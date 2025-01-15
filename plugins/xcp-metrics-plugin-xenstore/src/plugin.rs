mod metrics;

use std::collections::HashMap;

use async_stream::stream;
use compact_str::{CompactString, ToCompactString};
use futures::{Stream, StreamExt};
use radix_trie::Trie;
use tokio::net::UnixStream;
use uuid::Uuid;

use xcp_metrics_common::{
    metrics::{Label, MetricType},
    protocol::{CreateFamily, ProtocolMessage, RemoveMetric, UpdateMetric, XcpMetricsAsyncStream},
};
use xen::{domctl::DomctlGetDomainInfo, hypercall::XenHypercall};
use xenstore_rs::{tokio::XsTokio, AsyncWatch, AsyncXs};

use metrics::{MemInfoFree, MemInfoTotal, MetricHandler, MetricHandlerEnum};

#[derive(Default)]
struct PluginState {
    /// Map each domid-subpath with a UUID.
    metrics_map: HashMap<u16, HashMap<CompactString, Uuid>>,
    /// Map each domid with the domain's UUID.
    domid_uuid_map: HashMap<u16, Uuid>,
}

/// Split the path into: (Domain ID, subpath)
fn parse_path(path: &str) -> Option<(u16, &str)> {
    let path = path.strip_prefix("/local/domain/")?;

    // path now looks like <domid>[/<subpath>]
    let (domid_str, subpath) = if path.contains('/') {
        path.split_once("/")?
    } else {
        (path, "")
    };

    let domid = domid_str
        .parse()
        .inspect_err(|e| tracing::warn!("Invalid domid as subpath of /local/domain: {path} ({e})"))
        .ok()?;

    Some((domid, subpath))
}

async fn initialize_families(stream: &mut UnixStream) -> anyhow::Result<()> {
    stream
        .send_message_async(ProtocolMessage::CreateFamily(CreateFamily {
            help: "Total memory usable by the guest".into(),
            name: "xen_memory_usage_total".into(),
            metric_type: MetricType::Gauge,
            unit: "bytes".into(),
        }))
        .await?;

    stream
        .send_message_async(ProtocolMessage::CreateFamily(CreateFamily {
            help: "Free memory inside the guest".into(),
            name: "xen_memory_usage_free".into(),
            metric_type: MetricType::Gauge,
            unit: "bytes".into(),
        }))
        .await?;

    Ok(())
}

fn recursive_traversal(xs: XsTokio, path: String) -> impl Stream<Item = Box<str>> {
    stream! {
        yield path.clone().into_boxed_str();

        if let Ok(subpaths) = xs.directory(&path).await {
            for subpath in &subpaths {
                let entries = Box::pin(recursive_traversal(xs.clone(), format!("{path}/{subpath}")));
                for await entry in entries {
                    yield entry;
                }
            }
        }
    }
}

pub async fn run_plugin(
    mut stream: UnixStream,
    hyp: impl XenHypercall,
    xs: XsTokio,
) -> anyhow::Result<()> {
    initialize_families(&mut stream).await?;

    // First get all existing xenstore entries, and then use the watch.
    let mut domain_watcher = Box::pin(recursive_traversal(xs.clone(), "/local/domain".into()))
        .chain(xs.watch("/local/domain").await?);

    let mut handlers: Trie<&str, MetricHandlerEnum> = Trie::new();

    let meminfo_total = MemInfoTotal;
    handlers.insert(meminfo_total.subpath(), meminfo_total.into());

    let meminfo_free = MemInfoFree;
    handlers.insert(meminfo_free.subpath(), meminfo_free.into());

    let mut state = PluginState::default();

    while let Some(path) = domain_watcher.next().await {
        if path.as_ref() == "/local/domain" {
            continue;
        }

        // Parse the domid of the new domain.
        if let Some((domid, subpath)) = parse_path(&path) {
            // Don't try to read these entries as it can bug PV interfaces.
            if subpath.starts_with("device")
                || subpath.starts_with("backend")
                || subpath.starts_with("console")
            {
                continue;
            }

            let entry = xs.read(&path).await;

            // Check for /local/domain/<domid> paths.
            // When a domain dies, we only get the /local/domain/<domid> event,
            // and in such case, we need to remove all <domid> metrics.
            if subpath.is_empty() && entry.is_err() {
                // Get all related registered metrics.
                let entries = state.metrics_map.remove(&domid).unwrap_or_default();
                state.domid_uuid_map.remove(&domid);

                for (family_name, uuid) in entries {
                    stream
                        .send_message_async(ProtocolMessage::RemoveMetric(RemoveMetric {
                            family_name,
                            uuid,
                        }))
                        .await?;
                }

                continue;
            }

            // Update a metric.
            if let Ok(entry) = entry {
                let Some(handler) = handlers.get(subpath) else {
                    tracing::debug!("Ignoring {path}");
                    continue;
                };

                let Some(mut metric) = handler.read_metric(&xs, &path, subpath).await else {
                    tracing::warn!("No fetched metric from {path}={entry}");
                    continue;
                };

                // Insert domain uuid label.
                let domain_uuid = state.domid_uuid_map.entry(domid).or_insert_with(|| {
                    hyp.get_domain_info(xen::DomId(domid))
                        .inspect_err(|e| tracing::error!("get_domain_info failure: {e}"))
                        .map(|dominfo| dominfo.handle)
                        .unwrap_or_default()
                });

                let mut labels: Vec<Label> = metric.labels.into_vec();
                labels.push(Label {
                    name: "domain".into(),
                    value: domain_uuid.as_hyphenated().to_compact_string(),
                });
                metric.labels = labels.into_boxed_slice();

                let &mut uuid = state
                    .metrics_map
                    .entry(domid)
                    .or_insert_with(|| HashMap::new())
                    .entry(handler.family_name().into())
                    .or_insert_with(|| Uuid::new_v4());

                stream
                    .send_message_async(ProtocolMessage::UpdateMetric(UpdateMetric {
                        family_name: CompactString::const_new(handler.family_name()),
                        metric,
                        uuid,
                    }))
                    .await?;
            } else {
                // Remove the related metric (if there is)
                if let Some(uuid) = state
                    .metrics_map
                    .get_mut(&domid)
                    .and_then(|map| map.remove(subpath))
                {
                    let Some(handler) = handlers.get(subpath) else {
                        continue;
                    };

                    stream
                        .send_message_async(ProtocolMessage::RemoveMetric(RemoveMetric {
                            family_name: handler.family_name().into(),
                            uuid,
                        }))
                        .await?;
                }
            }
        } else {
            tracing::warn!("Unexpected watch event {path}")
        }
    }

    Ok(())
}

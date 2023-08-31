# xcp-metrics

xcp-metrics is a currently work in progress metrics daemon for xcp-ng.
It aims to support [OpenMetrics standard](https://github.com/OpenObservability/OpenMetrics) along with being (best-effort) compatible with original xcp-rrdd OCaml daemon.

## Project structure

```mermaid
flowchart LR
    common[xcp-metrics-common]
    click common "xcp-metrics-common"
    metrics[xcp-metrics]
    click metrics "xcp-metrics"
    plugin_common[xcp-metrics-plugins-common]
    click plugin_common "xcp-metrics-plugin-common"
    
    xapi[xapi-rs]
    click xapi "xapi-rs"

    squeezedp[xcp-metrics-plugin-squeezed]
    click squeezedp "plugins/xcp-metrics-plugin-squeezed"
    xenp[xcp-metrics-plugin-xen]
    click xenp "plugins/xcp-metrics-plugin-xen"
    xenstorep[xcp-metrics-plugin-xenstored]
    click xenstorep "plugins/xcp-metrics-plugin-xenstored"

    tools[xcp-metrics-tools]
    click tools "xcp-metrics-tools"
    rrdd(xcp-rrdd)

    squeezedp & xenp & xenstorep --- plugin_common

    legacy_rrdd_plugin -.-> v2
    common -.-> v2 & v3
    xapi -.-> metrics & rrdd
    plugin_common --- common & xapi
    tools ---- common & xapi

    v2((plugin\nprotocol v2)) -.- rrdd & metrics
    v3((plugin\nprotocol v3)) -.- metrics
```

## LICENSE

GNU Affero General Public License v3.0 only

https://spdx.org/licenses/AGPL-3.0-only.html
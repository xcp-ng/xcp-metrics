# xcp-metrics

xcp-metrics is a currently work in progress metrics daemon for xcp-ng.
It aims to support [OpenMetrics standard](https://github.com/OpenObservability/OpenMetrics) along with being (best-effort) compatible with original xcp-rrdd OCaml daemon.

## Project structure

```mermaid
flowchart LR
    common[<a href='./xcp-metrics-common'>xcp-metrics-common</a>]
    metrics[<a href='./xcp-metrics'>xcp-metrics</a>]
    plugin_common[<a href=./plugins/xcp-metrics-plugins-common'>xcp-metrics-plugins-common</a>]
    
    xapi[<a href='./xapi-rs'>xapi-rs</a>]

    squeezedp[<a href=./plugins/xcp-metrics-plugin-squeezed>xcp-metrics-plugin-squeezed</a>]
    xenp[<a href='./plugins/xcp-metrics-plugin-xen'>xcp-metrics-plugin-xen</a>]
    xenstorep[<a href='./plugins/xcp-metrics-plugin-xenstored'>xcp-metrics-plugin-xenstored</a>]

    tools[<a href='./xcp-metrics-tools'>xcp-metrics-tools</a>]
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
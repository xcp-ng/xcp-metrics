# xcp-metrics

xcp-metrics is a currently work in progress metrics daemon for xcp-ng.
It aims to support [OpenMetrics standard](https://github.com/OpenObservability/OpenMetrics) along with being (best-effort) compatible with original xcp-rrdd OCaml daemon.

## Project structure

```
.
├── metrics_sample : Protocol v2 snapshots for test using xcp-metrics-dump
├── plugins        : Various plugins and plugin framework
├── xcp-metrics    : Main daemon
├── xcp-metrics-common : xcp-metrics common library
├── xcp-metrics-test   : Scratch crate for tests
└── xcp-metrics-tools  : Various xcp-metrics utilities
```

## LICENSE

GNU Affero General Public License v3.0 only

https://spdx.org/licenses/AGPL-3.0-only.html
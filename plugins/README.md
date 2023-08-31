# xcp-metrics plugins

These plugins may be compatible with original xcp-rrdd and have their behavior altered when having their name prefixed with `rrdp-`.

## Libraries

### xcp-metrics-plugin-common

Provides a plugin framework that simplifies communication with daemon, conversions between protocol v2 and v3 facilities, and various other utilities to simplify plugin developement.

## Plugins

### xcp-metrics-plugin-bridge-v2

Utility plugin that converts over the air a protocol v3 plugin into a protocol v2 one. In this plugin, `uid` is suffixed with `_bridged`.

### xcp-metrics-plugin-squeezed

Squeezed plugin implementation compatible with OCaml plugin if suffixed with `rrdp-` (e.g `rrdp-squeezed`).

### xcp-metrics-plugin-xen

Xenctrl-based plugin. Superseeds OCaml `xcp-rrdd-xenpm` plugin.

### xcp-metrics-plugin-xenstored

XenStore-based plugin.

### xcp-metrics-plugin-tests

Collection of test plugins.
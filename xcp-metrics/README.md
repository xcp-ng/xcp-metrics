# xcp-metrics main daemon

This is the main xcp-metrics daemon that uses a RPC interface similar to xcp-rrdd (and what some other XAPI project uses).
In addition to support XML-RPC, it also supports JSON-RPC.

## Main modules

### forwarded

Forwarded implementation and routes (e.g `rrd_updates`) that manages the forwarded socket (e.g `xcp-rrdd.forwarded`).

### hub

Small module that aggregate metrics.

### providers

Metrics providers implementations (e.g protocol v2 and v3) that pushes metrics to hub.

### publishers

Modules that pulls metrics from hub and distribute them (using RPC, forwarded route or something else).

### rpc

RPC server implementation and routes.
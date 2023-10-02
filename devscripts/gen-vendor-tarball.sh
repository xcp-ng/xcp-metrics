#!/bin/sh
set -e

PACKAGE="xcp-metrics"
#VERSION=$(git describe --always "HEAD")
VERSION="$1"

# For this plugin, comment out patched xenstore-rs
CARGOTOML=plugins/xcp-metrics-plugin-common/Cargo.toml
awk  < $CARGOTOML > $CARGOTOML.patched '
  BEGIN {p=1};
  /^\[dependencies.xenstore-rs-wip\]/ {p=0};
  /^$/ {p=1};
  { if (p==1) {print} else { print "#" $0}}'
mv $CARGOTOML.patched $CARGOTOML
# remove reference to xenstore-rs-wip
sed -i '/^xenstore-wip = / d' \
      plugins/xcp-metrics-plugin-common/Cargo.toml

cargo vendor --versioned-dirs third-party
tar -zcf "../$PACKAGE-$VERSION-vendor.tar.gz" --xform="s,^,$PACKAGE-$VERSION/," third-party/

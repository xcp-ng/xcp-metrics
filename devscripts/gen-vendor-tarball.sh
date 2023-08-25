#!/bin/sh
set -e

PACKAGE="xcp-metrics"
#VERSION=$(git describe --always "HEAD")
VERSION=0.0.0

# # replace references to patched crates, for "cargo vendor"
# find -name Cargo.toml | xargs sed -Ei '/^(git|branch) = / s/^/#GEN#/'

cargo vendor --versioned-dirs third-party
tar -zcf "../$PACKAGE-$VERSION-vendor.tar.gz" --xform="s,^,$PACKAGE-$VERSION/," third-party/

# # unpatch references to patched crates
# find -name Cargo.toml | xargs sed -Ei 's/^#GEN#//'

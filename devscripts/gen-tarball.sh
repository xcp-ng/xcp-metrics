#!/bin/sh
set -e

# FIXME "git archive" wants $REF and "cargo vendor" wants $PWD

REF="$1"
PACKAGE="xcp-metrics"
#VERSION=$(git describe --always "$REF")
VERSION=0.0.0

# FIXME use "cargo package"?  Requires all of our crates to include a version :/
git archive "$REF" --prefix="$PACKAGE-$VERSION/" -o "../$PACKAGE-$VERSION.tar.gz"

#!/bin/sh
set -e

SOURCEBRANCH="main"
TARGETBRANCH="$1"

die () {
    echo >&2 "ERROR: $0: $*"
    exit 1
}

## prologue

# check for dirty workspace
git diff-index --quiet HEAD -- || die "dirty workspace"

# check Cargo.lock is uptodate enough
cargo build --offline
git diff-index --quiet HEAD -- || die "moving Cargo.lock"

## basics and xenctrl plugin

CRATES="
  plugins/xcp-metrics-plugin-xen
  plugins/xcp-metrics-plugin-common
  xcp-metrics-common
  xapi-rs
"

CRATES_RE=$(echo $CRATES | tr ' ' '|')

CONTENTS="
  $CRATES
  Cargo.toml
  Cargo.lock
  .gitignore
  .github
  LICENSE
  devscripts/gen-vendor-tarball.sh
  metrics_sample/xcp-rrdd-mem_host
  metrics_sample/xcp-rrdd-xenpm
"

# new branch for initial commit
git switch --orphan "$TARGETBRANCH"
# with specified contents
git restore -WS --source="$SOURCEBRANCH" $CONTENTS

# and only specified crates in workspace
sed -E -i Cargo.toml -e '/^[^ ]/ p
\,"('$CRATES_RE')", p
d'
git add Cargo.toml

# For this plugin, unpatched xenstore-rs is enough
sed -Ei '/^(git|branch) = / d' \
      plugins/xcp-metrics-plugin-common/Cargo.toml
git add plugins/xcp-metrics-plugin-common/Cargo.toml

# filter Cargo.lock to match source files
cargo build --offline
git add Cargo.lock

# commit
<<EOF cat |
WIP Introduce xcp-metrics-plugin-xen plugin for xcp-rrdd

xcp-metrics-plugin-xen is meant as a replacement for
- the metrics collection built into xcp-rrdd, which requires the
  latter to depend on libxenctrl
- the rrdp-pm plugin, which also fetches data through libxenctrl

Metrics are collected using the `xenctrl` Rust crate, structured in a
suitable way for upcoming OpenMetrics support, and communicated to
xcp-rrdd using its v2 protocol.

NOTE: at the time of this writing, we still rely on extra features we
had to add to the `xenctrl` crate, PR pending.

Signed-off-by: Teddy Astie <teddy.astie@outlook.fr>
Reviewed-by: Yann Dirson <yann.dirson@vates.fr>
EOF
git commit -F - --author=Teddy

# generate source tarball
PACKAGE="xcp-metrics"
VERSION=0.0.0
git diff-index --quiet HEAD -- || die "dirty workspace after committing $VERSION"
git archive HEAD --prefix="$PACKAGE-$VERSION/" -o "../$PACKAGE-$VERSION.tar.gz"

## xcp-metrics

CRATES="
  $CRATES
  xcp-metrics
  xcp-metrics-tools
"

CRATES_RE=$(echo $CRATES | tr ' ' '|')

CONTENTS="
  $CRATES
  Cargo.toml
  Cargo.lock
"

git restore -WS --source="$SOURCEBRANCH" $CONTENTS

# only specified crates in workspace
sed -E -i Cargo.toml -e '/^[^ ]/ p
\,"('$CRATES_RE')", p
d'
git add Cargo.toml

# unpatched xenstore-rs is still enough
sed -Ei '/^(git|branch) = / d' \
      plugins/xcp-metrics-plugin-common/Cargo.toml
git add plugins/xcp-metrics-plugin-common/Cargo.toml

# filter Cargo.lock to match source files
cargo build --offline
git add Cargo.lock

# commit
<<EOF cat |
WIP Introduce xcp-metrics daemon

xcp-metrics exposes the same kind of metrics that xcp-rrdd does, but
using the OpenMetrics standard.

It uses an reworked version of the v2 protocol currently in use by xcp-rrdd,
proposed as v3 protocol in xapi-project/xapi-project.github.io#278

Signed-off-by: Teddy Astie <teddy.astie@outlook.fr>
Reviewed-by: Yann Dirson <yann.dirson@vates.fr>
EOF
git commit -F - --author=Teddy

# generate source tarball
VERSION=0.0.1
git diff-index --quiet HEAD -- || die "dirty workspace after committing $VERSION"
git archive HEAD --prefix="$PACKAGE-$VERSION/" -o "../$PACKAGE-$VERSION.tar.gz"

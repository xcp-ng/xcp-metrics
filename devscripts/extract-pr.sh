#!/bin/sh
set -e

SOURCEBRANCH="main"
TARGETBRANCH="$1"

## basics and squeezed plugin

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
  metrics_sample/xcp-rrdd-squeezed
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

# commit
<<EOF cat |
WIP Introduce xcp-metrics-plugin-squeezed plugin for xcp-rrdd

xcp-metrics-plugin-squeezed is meant as a drop-in replacement for
rrdp-squeezed, as a first Rust-written brick for XAPI.

Metrics are read in Xenstore using the xenstore-rs bindings of
libxenstore, structured in a suitable way for upcoming OpenMetrics
support, and communicated to xcp-rrdd using its v2 protocol.

NOTE: while the full xcp-metrics work currently references external
git repository for xenstore-rs and xenctrl-rs as we had to add some
features there and the PRs are still pending, this initial work
explicitly does ot require any of those, and we went into some
gymnastics to support this, which reflects in the scripts extracting
this PR source code, as well as in parts of the code itself.

Signed-off-by: Teddy Astie <teddy.astie@outlook.fr>
Reviewed-by: Yann Dirson <yann.dirson@vates.fr>
EOF
git commit -F - --author=Teddy

# generate source tarball
PACKAGE="xcp-metrics"
VERSION=0.0.0
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
git archive HEAD --prefix="$PACKAGE-$VERSION/" -o "../$PACKAGE-$VERSION.tar.gz"

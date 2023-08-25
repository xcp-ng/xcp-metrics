#!/bin/sh
set -e

TARGETBRANCH="$1"

CRATES="
  plugins/xcp-metrics-plugin-squeezed
  plugins/xcp-metrics-plugin-common
  xcp-metrics-common
"

CRATES_RE=$(echo $CRATES | tr ' ' '|')

CONTENTS="
  $CRATES
  Cargo.toml
  .gitignore
  .github
  LICENSE
  devscripts/gen-tarball.sh
  devscripts/gen-vendor-tarball.sh
"

# new branch for initial commit
git switch --orphan "$TARGETBRANCH"
# with specified contents
git restore -WS --source="main" $CONTENTS

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

NOTE: this currently references external git repository for
xenstore-rs, as we had to add support for xs_watch there and the PR is
still pending.

Signed-off-by: Teddy Astie <teddy.astie@outlook.fr>
Reviewed-by: Yann Dirson <yann.dirson@vates.fr>
EOF
git commit -F - --author=Teddy

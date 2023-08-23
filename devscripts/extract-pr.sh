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

# commit
<<EOF cat |
WIP Introduce xcp-metrics-plugin-squeezed plugin for xcp-rrdd

xcp-metrics-plugin-squeezed is meant as a drop-in replacecement for
rrdp-squeezed, as a first Rust-written brick for XAPI.

EOF
git commit -S -F - --author=Teddy

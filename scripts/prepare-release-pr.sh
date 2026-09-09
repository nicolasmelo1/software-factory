#!/bin/sh
# Complete the artifacts derived from Cargo.toml after release-plz creates or
# refreshes its release PR. release-plz owns the version and changelog; this
# script owns this repository's declared derivatives of that version.
set -eu

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml)
if [ -z "$version" ]; then
  echo "could not read the package version from Cargo.toml" >&2
  exit 1
fi

python3 - "$version" <<'PY'
from pathlib import Path
import re
import sys

path = Path("README.md")
content = path.read_text()
updated, replacements = re.subn(
    r"(--tag v)[0-9]+\.[0-9]+\.[0-9]+",
    rf"\g<1>{sys.argv[1]}",
    content,
    count=1,
)
if replacements != 1:
    raise SystemExit("README.md must contain exactly one cargo install tag")
path.write_text(updated)
PY

cargo build --release --locked
./target/release/sf fixtures
./target/release/sf docs
./target/release/sf ratchet
./target/release/sf lock

cargo test --locked
./target/release/sf verify --allow-commands
./target/release/sf check --allow-commands

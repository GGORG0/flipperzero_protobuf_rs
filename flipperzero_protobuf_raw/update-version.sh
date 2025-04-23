#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$(realpath "$0")")" || exit

CHANGELOG="src/proto-src/Changelog"
CARGO_TOML="Cargo.toml"
PROTO_SRC="src/proto-src"

latest_major_minor=$(grep -oP '^## \[\K[0-9]+\.[0-9]+' "$CHANGELOG" | head -n1)
semver_version="${latest_major_minor}.0"

commit_hash=$(git -C "$PROTO_SRC" rev-parse --short HEAD)

new_version="${semver_version}+${commit_hash}"

sed -i -E "s/^version = \".*\"/version = \"${new_version}\"/" "$CARGO_TOML"

echo "Updated Cargo.toml to version = \"$new_version\""

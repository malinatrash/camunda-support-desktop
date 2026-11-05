#!/usr/bin/env bash
# Собирает NSIS-установщик Windows в target/release/bundle/nsis.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
cargo build --release --package camunda-support
if ! command -v makensis >/dev/null; then
  choco install nsis --yes --no-progress
  export PATH="/c/Program Files (x86)/NSIS:$PATH"
fi
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
mkdir -p target/release/bundle/nsis
makensis -V2 "-DVERSION=$version" packaging/windows.nsi

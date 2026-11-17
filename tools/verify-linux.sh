#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
bash tools/verify-licenses.sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo clippy --workspace --all-targets --no-default-features --target-dir target/linux-cli --locked -- -D warnings
cargo test --workspace --no-default-features --target-dir target/linux-cli --locked
cargo build --locked
dbus-run-session -- xvfb-run -a -s '-screen 0 1280x800x24' python3 tools/smoke-linux.py
printf 'Verified Linux core, host, CLI and isolated desktop integration.\n'

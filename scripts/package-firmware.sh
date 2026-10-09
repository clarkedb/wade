#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec cargo run --quiet --locked -p wade-xtask -- package-firmware "$@"

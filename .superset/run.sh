#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" == Darwin ]] && command -v brew >/dev/null; then
  export LIBRARY_PATH="${LIBRARY_PATH:+$LIBRARY_PATH:}$(brew --prefix)/lib"
fi

exec cargo run -p wade-desktop -- --seed "${WADE_SEED:-42}"

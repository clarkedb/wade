#!/usr/bin/env bash
set -euo pipefail

git config core.hooksPath .githooks

# Installs the channel, components, and targets pinned in rust-toolchain.toml.
rustup toolchain install

if [[ "$(uname -s)" == Darwin ]] && ! brew list sdl2 >/dev/null 2>&1; then
  echo "SDL2 is missing; wade-desktop needs it: brew install sdl2" >&2
fi

cargo fetch --locked

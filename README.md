# wade

my desk buddy

Wade is an animated desk companion for the M5Stack CoreS3 Lite, developed and tested on desktop first. Design docs live in [docs/](docs/README.md); the plan is in [docs/roadmap.md](docs/roadmap.md).

## Layout

| Path | Contents |
|---|---|
| `wade-core/` | `no_std`, allocation-free core: behavior, layout, drawing |
| `wade-desktop/` | Desktop simulator (SDL2 window, record and replay) |
| `wade-cores3/` | Firmware for the device (from M3; outside the workspace) |
| `spikes/` | Throwaway experiments (outside the workspace) |

## Setup

The toolchain is pinned in `rust-toolchain.toml`; `rustup` installs it on first use.

```sh
brew install sdl2
# On Apple Silicon, if linking fails to find SDL2:
export LIBRARY_PATH="$LIBRARY_PATH:$(brew --prefix)/lib"
```

## Run

```sh
cargo run -p wade-desktop -- --seed 42
```

Flags: `--seed <n>`, `--record <file>`, `--replay <file>`, `--time-scale <x>`.

## Checks

Run before every commit; CI runs the same ones.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build -p wade-core --target thumbv7em-none-eabihf
! grep -rn "extern crate alloc" wade-core/src
```

`UPDATE_SNAPSHOTS=1 cargo test` rewrites golden images in `wade-core/tests/snapshots/`.

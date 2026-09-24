---
name: verify
description: Run wade's local verification (format, lints, no_std and no-alloc guards, tests) and, when behavior changed, run the desktop app. Use before committing, before opening a PR, or when asked to verify, check, or test changes.
---

# Verify

Run the checks CI runs, then the tests, and report what passed and what failed with the relevant output.

## Environment

On Apple Silicon, anything that links SDL2 (tests, `cargo run`) needs:

```sh
export LIBRARY_PATH="${LIBRARY_PATH:+$LIBRARY_PATH:}$(brew --prefix)/lib"
```

## Checks

From the repo root, in order; stop and fix at the first failure:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy -p wade-core --lib --target thumbv7em-none-eabihf --locked -- -D warnings
! grep -rnE 'extern[[:space:]]+crate[[:space:]]+alloc' wade-core/src
cargo test --workspace --locked
```

The bare-metal clippy is the only guard that `wade-core` stays `no_std` (workspace builds unify it with `std`), and the grep is the only guard against `alloc`. Never skip either.

`cargo fmt --all` and `cargo clippy --fix --allow-dirty` fix most format and lint failures.

## Snapshots and recordings

A snapshot failure writes `<name>.actual.png` beside the golden image. If the pixel change is intended, `UPDATE_SNAPSHOTS=1 cargo test` rewrites the goldens; for replay hash changes, `UPDATE_RECORDINGS=1 cargo test`. Say which goldens or recordings changed and why; never regenerate them just to make a failure go away.

## Runtime

When a change affects behavior or drawing, also run the simulator and confirm it works:

```sh
cargo run -p wade-desktop -- --seed 42
```

## Scope

Docs-only or config-only changes need no cargo steps. Firmware in `wade-cores3/` builds separately with the Espressif toolchain (`cd wade-cores3 && cargo build --release`); only run it when that crate changed and the toolchain is installed.

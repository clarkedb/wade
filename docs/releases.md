# Releases

Wade has one semantic product version across desktop and both supported boards.
`version.txt` records it; workspace members inherit it, and standalone firmware
manifests and lockfiles must agree. Rust crates are not published.

Run `scripts/check-version.sh` to check the version with the pinned host Rust
toolchain. CI runs the same check. Use Conventional Commits so release tooling
can derive version bumps from the commit history.

# Contributing to casc-extractor

## Getting started

**Prerequisites**

- Rust 1.70+
- cmake (for building CascLib)
- CascLib shared library — build from source and place in `lib/`. See [`lib/README.md`](lib/README.md) for full instructions.

**Build**

```bash
cargo build
```

**Run**

```bash
DYLD_LIBRARY_PATH=lib ./target/debug/casc-extractor --help
```

On Linux use `LD_LIBRARY_PATH=lib` instead.

## Running tests

All 156 unit tests are pure Rust and require no game data or libcasc:

```bash
DYLD_LIBRARY_PATH=lib DYLD_FRAMEWORK_PATH=lib cargo test --lib
```

To check compilation of all targets without linking libcasc:

```bash
cargo check --all-targets
```

Linting must pass cleanly before submitting:

```bash
cargo clippy --all-targets -- -D warnings
```

## CI

GitHub Actions runs on every push and PR to `main`. The pipeline:

1. Builds libcasc from source
2. `cargo check --all-targets`
3. `cargo test --lib`
4. `cargo clippy --all-targets -- -D warnings`

All steps must pass before a PR can be merged.

## Project structure

```
src/
├── main.rs          — CLI entry point (clap); all subcommand handlers
├── casc/            — CascLib FFI, archive discovery, file enumeration
├── anim/            — HD ANIM format parser and frame export
├── grp/             — SD GRP sprite parser and RLE decoder
├── sprite/          — extraction pipeline and PNG spritesheet builder
├── config/          — ExtractionConfig (JSON-serializable via serde)
├── validation/      — byte/visual comparison, regression suite
├── filter/          — include/exclude regex filtering
└── resolution/      — domain types for quality levels
```

## Adding a new extraction command

1. Add a variant to the relevant `Commands` or `ExtractCommands` enum in `src/main.rs`.
2. Write a handler function (e.g. `fn handle_extract_foo(...) -> Result<()>`).
3. Wire it into the `match` block in `main()`.

Keep handler functions focused — if the logic exceeds ~60 lines, pull shared logic into a helper.

## Code style

- Run `cargo clippy -- -D warnings` before committing. No warnings, no exceptions.
- Do not add `#[allow(dead_code)]` without a comment explaining why the item must be kept.
- Keep functions under ~60 lines; extract helpers if needed.
- Do not add docstrings or comments to code you did not change.

## Submitting changes

1. Fork the repo and create a feature branch.
2. Make your changes; ensure `cargo test --lib` and `cargo clippy` both pass.
3. Open a PR to `main`. Describe what was changed and why.
4. CI must be green before the PR will be reviewed.

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
./target/debug/casc-extractor --help
```

No loader environment variables are needed: `build.rs` and `.cargo/config.toml`
embed rpaths that resolve CascLib relative to the executable (`@loader_path` on
macOS, `$ORIGIN` on Linux). If you copy the bare binary elsewhere, copy `lib/`
beside it.

## Running tests

```bash
cargo test --lib      # 105 unit tests, no game data required
cargo test --tests    # integration suites, incl. the pixel baselines
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

### Pixel baselines

`tests/baselines/validation-suite.json` pins the SHA256 of nine real outputs
covering every decode path (HD BC3 diffuse, BC1 team-colour masks, an SD sprite, a
tileset atlas, a map terrain render). `tests/pixel_regression_test.rs` regenerates
them from the archive and compares.

It no-ops without a local archive, so **run it before submitting anything that
touches decoding, encoding or terrain**:

```bash
cargo test --test pixel_regression_test        # uses /Applications/StarCraft
STARCRAFT_PATH=/path/to/StarCraft cargo test --test pixel_regression_test
```

If your change is *meant* to alter output, confirm the pixels are what you intend,
then re-register the affected files:

```bash
casc-extractor validate register <file> --suite tests/baselines/validation-suite.json
```

Do not re-register to make a red test go green without checking the images first —
that is exactly what these baselines exist to prevent. Several past bugs produced
output that looked entirely plausible.

## CI

GitHub Actions runs on every push and PR to `main`, as a matrix over
ubuntu-latest and macos-latest (macOS matters: it is the only platform where
CascLib builds as a framework):

1. Build libcasc from source — framework layout on macOS, flat `.so` on Linux
2. `cargo fmt --check`
3. `cargo check --all-targets`
4. `cargo test --lib` and `cargo test --tests`
5. Assert the binary runs with all loader env vars unset
6. `cargo clippy --all-targets -- -D warnings`
7. Smoke-test the CLI surface

`cargo audit` runs as a separate job and is a hard gate — not `continue-on-error`.
All third-party actions are pinned to commit SHAs; if you bump one, pin the new SHA
and keep the tag as a trailing comment.

All steps must pass before a PR can be merged.

## Project structure

```
src/
├── main.rs          — clap definitions + dispatch ONLY (no handler logic)
├── commands/        — handlers, grouped by subject
│   ├── support.rs   — path safety, overwrite policy, config, archive opening,
│   │                  atomic writes
│   ├── sprites.rs   — extract anim / tileset / effect
│   ├── terrain.rs   — tileset atlases, map-terrain, VR4 geometry, CHK parsing
│   ├── assets.rs    — cmdicons, mainmenu, dat, raw
│   ├── sounds.rs    — sounds extract / list / probe / export-targets
│   ├── inspect.rs   — inspect sprites / archive / files
│   └── maintenance.rs — config init, validate register / run
├── casc/            — CascLib FFI, archive discovery, file enumeration
├── anim/            — HD ANIM parser (0x0202/0x0204) + SD ANIM (0x0101)
├── grp/             — SD GRP sprite parser and RLE decoder
├── sprite/export.rs — ExportConfig, export_anim pipeline
├── dds_converter.rs — DDS → PNG (BC1/BC3), atomic write helpers
├── config/          — ExtractionConfig + FormatFilterOption
└── validation/      — byte/visual comparison, regression suite
tests/baselines/     — committed SHA256 suite guarding pixel output
```

## Adding a new extraction command

1. Add a variant to the relevant `Commands` or `ExtractCommands` enum in
   `src/main.rs`. Bound any numeric argument that will be divided by or
   multiplied (`value_parser!(u32).range(1..=4096)`) — unbounded `--cols` and
   `--tile-size` previously panicked.
2. Write a `pub fn` handler in the matching `src/commands/*.rs` module. Do not put
   handler logic in `main.rs`.
3. Wire it into the `match` block in `main()`.
4. Add it to `every_documented_subcommand_resolves` in
   `tests/integration_tests.rs`, and document it in the README.

Keep handler functions focused — if the logic exceeds ~60 lines, pull shared logic
into a helper. Prefer extending `commands/support.rs` over copy-pasting between
modules: duplicated CHK and tileset code is why several bugs had to be fixed twice.

### Writing output

Use `write_atomic` / `save_rgba_as_png` rather than `fs::write` or `File::create`
for anything a user keeps. A direct write can leave a truncated file, and the
`Never` / `IfNewer` overwrite modes then treat it as completed work.

Route user-supplied names through `join_within` before joining them onto the output
directory — `--name-map` and `--targets` values come from JSON and could otherwise
escape via `..`.

## Code style

- Run `cargo clippy -- -D warnings` before committing. No warnings, no exceptions.
- There are currently **zero** `#[allow(dead_code)]` in the tree; keep it that way.
  If something is test-only, gate it on `#[cfg(test)]` so the intent is explicit
  rather than suppressing the warning.
- Keep functions under ~60 lines; extract helpers if needed.
- Do not add docstrings or comments to code you did not change.

## Submitting changes

1. Fork the repo and create a feature branch.
2. Make your changes; ensure `cargo test --lib`, `cargo test --tests`,
   `cargo clippy` and `cargo fmt --check` all pass. Run the pixel regression test
   if you touched decoding, encoding or terrain.
3. Open a PR to `main`. Describe what was changed and why.
4. CI must be green before the PR will be reviewed.

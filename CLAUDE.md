# Notes for AI agents working on casc-extractor

Read `CONTRIBUTING.md` for build/test/CI mechanics and the project layout. This file
covers only what is non-obvious and has caused real bugs here.

## Run it plainly — no loader env vars

```bash
cargo build --release
./target/release/casc-extractor inspect archive
```

Do **not** prefix commands with `DYLD_LIBRARY_PATH=lib DYLD_FRAMEWORK_PATH=lib` or
`LD_LIBRARY_PATH=lib`. `build.rs` and `.cargo/config.toml` embed rpaths that resolve
CascLib relative to the executable, verified for `cargo run`, `cargo test` and
`target/release/`. Older notes in this repo's history say otherwise; they were wrong.

The archive auto-discovers at `/Applications/StarCraft`. Override with
`--install-path`, or set `STARCRAFT_PATH` for tests.

## The single most important rule: verify pixels, don't infer them

Every serious bug found in this codebase produced **plausible-looking wrong output**.
A lava map rendered as blue-grey jungle stone for two independent reasons at once. A
tileset atlas built with the wrong stride still produced a neat grid — of noise.
Nothing crashed; no test failed.

So if you change decoding, encoding, palettes or terrain:

1. Run `cargo test --test pixel_regression_test` (needs a local archive; it no-ops
   without one, which means a green run on CI proves nothing).
2. Actually look at the output. Render a crop and view it.
3. Diff byte-exactly against an independent decoder when one exists
   (`magick foo.dds ref.png`, then compare `Image.tobytes()`), and report
   differing-pixel counts and max per-channel delta — not just pass/fail.

If a change is *meant* to alter output, confirm the images are what you intend
**before** re-registering baselines:

```bash
casc-extractor validate register <file> --suite tests/baselines/validation-suite.json
```

Never re-register just to turn a red test green.

## Confirm what you are looking at

Two false alarms happened during past work, both from bad references rather than bad
code: an SD sprite that looked broken was a genuinely near-monochrome shadow sprite,
and a "layer mismatch" was a wrong index guess in the test harness. Before concluding
a bug exists:

- Get authoritative bytes from the project's own parser, not by guessing offsets.
- Check whether the symptom also reproduces on the pre-change build
  (`git stash` + rebuild) before assuming you caused it.
- Watch your own shell quoting. Two "everything FAILED" sweeps were word-splitting
  in the test loop, not the binary.

## Archive format facts that the code was wrong about

These are verified against the real archive. Trust them over any comment that
disagrees, but re-verify with a hexdump if a new tileset or quality misbehaves.

- **`tileset/*.dds.vr4` is not one big DDS.** It is a 20-byte outer header followed
  by N standalone DDS files. Stride = 128-byte DDS header + BC1 payload + 12 bytes
  padding, and tile size varies by quality: 128×128/8332B at full res, 64×64/2188B
  under HD2. Never hardcode either; `vr4_layout()` reads them from the first
  embedded header.
- **`tileset/*.wpe` is RGB0, not BGR0.** Check: ashworld entry 3 is `eb 53 17` —
  orange lava as RGB, absurd navy as BGR.
- **CHK `ERA ` ids (low 3 bits):** `0=badlands 1=platform 2=install 3=ashworld
  4=jungle 5=desert 6=ice 7=twilight`. `install` at 2 is easy to miss because no
  ladder map ships with it.
- **A MTXM id is not a tile index.** Resolve through CV5 first:
  `CV5[(id >> 4) * 52 + 20 + (id & 0xf) * 2]` as u16. Skipping this leaves ~70% of
  lookups out of range. For HD, that resolved index addresses a whole 128×128
  megatile directly — there is no 4×4 mini-tile assembly step.
- **The `teamcolor` layer is BC1 with punchthrough alpha.** All its blocks use
  `color0 <= color1`, where colour index 3 means transparent.
  `texture2ddecoder::decode_bc1` does *not* emit that alpha, so without
  `apply_bc1_punchthrough_alpha` every pixel reads as team-coloured.
- **Real HD layer order** (from the file header, id 025):
  `0 diffuse DXT5 | 1 bright DXT5 | 2 teamcolor DXT1 | 3 emissive DXT5 |
  4 normal DXT5 | 5 specular DXT1 | 6 ao_depth DXT5`. Note normal is DXT5 and
  specular is DXT1 — easy to swap when guessing. Header names are present in real
  files, so name lookup beats any index table.
- **Team colour**: multiply the *raw* diffuse by the player colour where the mask is
  set. Do not pre-process the diffuse; see `docs/team-color.md`.
- **`CascFindData` must stay byte-exact** with CascLib's `CASC_FIND_DATA` (1104
  bytes) or `sounds`/`list` SIGSEGV. Verify against upstream headers, and prefer the
  implementation over header comments — the online params separator is `*`, not the
  `:` the header's own comment implies.

## Writing output

Use `write_atomic` / `save_rgba_as_png`, never bare `fs::write` or `File::create`,
for anything a user keeps: a truncated file would be treated as completed work by the
`Never` / `IfNewer` overwrite modes.

Route user-supplied names through `join_within` before joining onto the output
directory. `--name-map` and `--targets` values come from JSON and can otherwise
escape via `..`.

## Performance notes

- `png_compression_level` applies to every PNG and is the dominant cost of a run.
  Default is 1 deliberately: over three sprites, level 1 is 0.7s/62 MB, level 9 is
  3.1s/59 MB. Pixels are identical at every level. Re-measure before changing it.
- DDS decoding is in-process (`ddsfile` + `texture2ddecoder`). Do not reintroduce an
  ImageMagick subprocess; it was ~62x slower.
- `image`'s default features pull in AVIF encoders this tool never uses and dominate
  build time — keep it restricted to `png`+`jpeg`.

## House style

- Zero `#[allow(dead_code)]` in the tree. Gate test-only items on `#[cfg(test)]`.
- `main.rs` is clap definitions plus dispatch only; handlers go in `src/commands/`.
- Prefer extending `commands/support.rs` over copy-pasting between command modules.
  Duplicated CHK and tileset code is why several bugs had to be fixed in two places.
- `cargo fmt` and `cargo clippy --all-targets -- -D warnings` must be clean.

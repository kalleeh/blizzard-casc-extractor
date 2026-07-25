# Changelog

All notable changes to `casc-extractor` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- SD ANIM parsing panicked on a slice index when the header's `layer_count`
  exceeded the ten reserved name slots; a single malformed sprite also failed the
  entire 38 MB `mainSD.anim`, discarding all ~999 sprites.
- HD layer extraction allocated from an unvalidated `u32` size before any bounds
  check, so a corrupt header could request up to 4 GiB.
- `extract raw --online` never contacted a CDN: it passed `0x40` as a "feature
  flag" to `CascOpenStorage`, whose second parameter is `dwLocaleMask` — `0x40`
  is `CASC_LOCALE_ZHCN`. It now uses `CascOpenOnlineStorage` with a proper
  `cache*product*region` params string and a region read from `.build.info`.
- Failed archive enumeration was reported as an empty-but-successful listing
  (`CascFindFirstFile` signals failure with `INVALID_HANDLE_VALUE`, not NULL).
- `CascGetFileSize` failure surfaced as a bogus "size 4294967295 exceeds maximum".
- Published release artifacts could not start at all — the bare binary shipped
  without CascLib, and the embedded rpaths only resolve from `target/<profile>/`.
  Releases now ship a tarball containing `lib/`, verified by running the packaged
  binary before publishing.
- `--validate-only` is documented as global but was only honoured by `extract`,
  so `sounds extract --validate-only` wrote files.
- `inspect files --sizes` printed bare paths (both branches were identical).
- `extract tileset-megatile --quality` was accepted and ignored.
- `OverwriteBehavior::Backup` overwrote the previous backup on every run, so only
  one generation ever survived.
- Multiple `MTXM` chunks in a CHK were concatenated instead of last-writer-wins.
- `build.rs` emitted Mach-O `@loader_path` rpaths on all platforms, leaving Linux
  builds with no usable rpath (ELF needs `$ORIGIN`).
- Terrain rendered with the wrong tileset for every map with `ERA >= 2`: BW's
  canonical ids are `2=install, 3=ashworld, 4=jungle`, so lava maps were drawn
  with jungle art.
- WPE palettes are RGB0, not BGR0 — red and blue were swapped in all
  `map-terrain` output.
- `.dds.vr4` tile stride was hardcoded to 8192. Each tile is a standalone DDS
  (128-byte header + BC1 payload + 12 padding) and its size varies by quality
  (128×128 full-res, 64×64 under HD2), so `tileset-atlas`, `tileset-megatile`
  and `map-terrain-dual` produced garbled atlases.
- `map-terrain-dual` skipped the CV5 indirection (`MTXM → CV5 → tile`) that
  `map-terrain` already performed, leaving 70% of tile lookups out of range.
- GRP RLE mis-decoded the code byte `0x40`, writing 64 wrong pixels and
  desynchronising the rest of the scanline.
- `extract raw --search` ANDed the `--locale` default into every path, hiding
  93% of the archive; locale now narrows only locale-scoped paths.
- `extract anim` with a name map copied rather than renamed each `.anim`,
  doubling output size.
- `--layers ao` silently produced nothing (header spells it `ao_depth`), and the
  conventional layer-index fallback table was misordered.
- `validate run` exited 0 on failing checks; `sounds extract` counted
  pre-existing files as freshly extracted.
- Path traversal via `--name-map` and `--targets` output names.
- Panics on `--cols 0`, oversized `--cols`/`--tile-size`, and non-ASCII
  `--targets` names.

### Changed

- `main.rs` split from ~4300 lines into `src/commands/` grouped by subject; it now
  holds only the CLI definition and dispatch.
- Removed 12 config fields that were deserialized but never read (most of
  `UnityExportSettings`, several `MetadataOptions`, both `resolution_tier` copies,
  `custom_settings`). Legacy config files still load; a test pins that.
- Deleted the unused `FileFilter` engine and `ValidationResult`; `src/filter`
  collapsed into a single enum now living in `config`.
- Dependencies moved to current majors: image 0.25 (png+jpeg features only, since
  the defaults pull in unused AVIF encoders), png 0.18, thiserror 2, ddsfile 0.6,
  indicatif 0.18. This also removed duplicate `bitflags` and `syn` copies.
- `calculate_file_size` no longer rescans every index per entry (was quadratic),
  and `sounds extract` enumerates the archive at most once per run.

- DDS → PNG decoding is done in-process (BC1/BC3 via `ddsfile` +
  `texture2ddecoder`) instead of spawning ImageMagick per file. A full SD run
  (868 sprites) goes from ~12.5s to ~0.9s; output is byte-identical. ImageMagick
  is no longer a dependency.
- `--team-color-mask` now writes the diffuse PNG unmodified alongside the binary
  mask, matching the documented shader. Previously masked pixels were replaced
  with BT.601 luminance, discarding chroma.
- Release profile enables `overflow-checks` and keeps the symbol table
  (`strip = "debuginfo"`) so parser panics yield usable backtraces.

### Added

- `tests/baselines/validation-suite.json` — committed SHA256 baselines for nine
  representative outputs covering every decode path, checked end-to-end by
  `tests/pixel_regression_test.rs`. The regression suite previously had no
  committed data, so no pixel regression could ever be caught.
- CI now also runs on macOS (the primary development platform, and the only one
  where CascLib builds as a framework), gates `cargo fmt --check`, and asserts the
  binary runs with all loader environment variables unset.

### Security

- `lz4_flex` 0.11.5 → 0.11.6 (RUSTSEC-2026-0041, CVSS 8.2: uninitialised memory
  disclosure when decompressing invalid data, reachable from ANIM textures) and
  `crossbeam-epoch` 0.9.18 → 0.9.20. CI's `cargo audit` is no longer
  `continue-on-error`, and now also runs `cargo test --tests`.

## [0.1.0] - 2026-06-14

Initial public release: a CASC extraction CLI for StarCraft: Remastered, with a
production-readiness pass for correctness, robustness, and operability.

### Added

- Unified `casc-extractor` CLI (clap-based) with structured subcommands and
  CASC archive auto-discovery.
- HD ANIM sprite extraction (versions 0x0202 / 0x0204) with layer export and
  team-colour compositing.
- SD ANIM sprite extraction (version 0x0101, palettised).
- Legacy GRP sprite parsing.
- Tileset extraction (`.dds.vr4`) with DDS→PNG conversion (handles the 20-byte
  VR4 header preceding the DDS magic).
- Terrain extraction with a per-mini-tile walkability (walk) map.
- Unit/UI sound (WAV) extraction; `sounds --fail-on-incomplete` flag to exit
  non-zero when a requested sound target is missing, for reliable scripted use.
- DAT file extraction.
- `--save-dds`, `--layers`, and `--team-color-mask` options on `extract anim`;
  `--convert-to-png` wired on `extract tileset` / `extract effect`.
- Unity integration options (pixels-per-unit, metadata JSON, filter/wrap modes).
- JSON configuration file support with overwrite-behaviour modes
  (Never / IfNewer / Backup / Prompt / Always).
- SHA256 byte-level validation suite.
- `docs/team-color.md`: full HD ANIM layer structure, the correct team-colour
  compositing formula, the SC1 player-colour table, and a reference implementation.

### Fixed

- Dark templar sprite corruption.
- Sprite-reference resolution and ZLIB/DEFLATE BLTE decompression correctness.
- Panic guards for malformed/truncated input so corrupt archives fail gracefully
  instead of aborting.
- Environment-variable handling made thread-safe.
- macOS test runs no longer require manual `DYLD_*` exports (rpath fix for tests).
- `get_team_color_layer()` now selects the "teamcolor" layer by name instead of a
  hardcoded index, fixing `--team-color-mask` output.
- Parallel DDS→PNG race condition in rayon threads (atomic temp-file counter).

[Unreleased]: https://github.com/kalleeh/blizzard-casc-extractor/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/kalleeh/blizzard-casc-extractor/releases/tag/v0.1.0

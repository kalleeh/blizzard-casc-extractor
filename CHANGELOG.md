# Changelog

All notable changes to `casc-extractor` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.0] - 2026-06-14

This is a production-readiness round focused on correctness, robustness, and
operability on top of the initial feature set.

### Added

- Terrain extraction now emits a per-mini-tile walkability (walk) map.
- Additional DAT file extraction targets.
- `sounds --fail-on-incomplete` flag: exit non-zero when one or more requested
  sound targets cannot be extracted, for reliable CI/scripted use.
- `--save-dds`, `--layers`, and `--team-color-mask` options on `extract organized`;
  `--convert-to-png` wired on `extract tileset` / `extract effect`.
- `docs/team-color.md`: full HD ANIM layer structure, the correct team-colour
  compositing formula, the SC1 player-colour table, and a reference implementation.

### Changed

- Validation pipeline simplified to SHA256-only byte-level comparison; removed
  no-op reference-tool plumbing.
- Config `format_filter = "Png"`/`"Images"` now implies `--convert-to-png` on all
  `extract` subcommands.
- Removed ~5000 lines of dead code (research, format_converter, format_analyzer,
  blte_enhanced, and cli modules; the unused DirectSpriteExtractor path).

### Fixed

- Dark templar sprite corruption.
- Sprite-reference resolution and ZLIB/DEFLATE BLTE decompression correctness.
- Panic guards for malformed/truncated input so corrupt archives fail gracefully
  instead of aborting.
- Environment-variable handling made thread-safe.
- macOS test runs no longer require manual `DYLD_*` exports (rpath fix for tests).
- `get_team_color_layer()` returned the wrong layer ("bright" at index 1 instead of
  "teamcolor" at index 2), affecting `--team-color-mask` output.
- Tileset PNG conversion now handles the 20-byte VR4 header preceding the DDS magic.
- Parallel DDS→PNG race condition in rayon threads (atomic temp-file counter).

## [1.0.0] - 2026-01-01

Initial release.

### Added

- Unified `casc-extractor` CLI (clap-based) with structured subcommands and
  CASC archive auto-discovery.
- HD ANIM sprite extraction (versions 0x0202 / 0x0204) with layer export and
  team-colour compositing.
- SD ANIM sprite extraction (version 0x0101, palettised).
- Legacy GRP sprite parsing.
- Tileset extraction (`.dds.vr4`) with DDS→PNG conversion.
- Unit/UI sound (WAV) extraction.
- DAT file extraction.
- Unity integration options (pixels-per-unit, metadata JSON, filter/wrap modes).
- JSON configuration file support with overwrite-behaviour modes
  (Never / IfNewer / Backup / Prompt / Always).

[Unreleased]: https://github.com/kalleeh/blizzard-casc-extractor/compare/v1.1.0...HEAD
[1.1.0]: https://github.com/kalleeh/blizzard-casc-extractor/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/kalleeh/blizzard-casc-extractor/releases/tag/v1.0.0

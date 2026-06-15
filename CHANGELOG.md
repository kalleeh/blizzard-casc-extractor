# Changelog

All notable changes to `casc-extractor` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- `--save-dds`, `--layers`, and `--team-color-mask` options on `extract organized`;
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

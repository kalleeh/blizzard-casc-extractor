# casc-extractor

Extract sprites and audio from StarCraft: Remastered CASC archives.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Supported Games

**StarCraft: Remastered** — fully tested.

Other CASC-based games are compatible but untested: Warcraft III: Reforged,
Heroes of the Storm, World of Warcraft, Diablo III, Overwatch.

*Other games may require format-specific parsers.*

## What Gets Extracted

| Content | Count | Notes |
|---------|-------|-------|
| HD sprites (4x) | 999 | ~4.5 MB each as raw ANIM |
| HD sprites (2x) | 999 | ~1.1 MB each |
| SD sprites | 868 | GRP palettised, named from mapping file |
| Tilesets | 8 | `.dds.vr4` with optional PNG conversion |
| Effects | 2 | Water normals |
| Sounds | 13/15 | WAV → OGG; 2 paths not present in this SC:R build |

### Quality Levels

| Level | CASC prefix | Format |
|-------|-------------|--------|
| `hd4` (default) | _(none)_ | ANIM v0x0202/0x0204 with DXT5 textures |
| `hd2` | `HD2/` | Same format, lower resolution |
| `sd` | `SD/` | ANIM v0x0101, GRP palettised sprites |

## Requirements

- Rust 1.70+
- StarCraft: Remastered installed
- CascLib native library (must be built from source — see [`lib/README.md`](lib/README.md))
- macOS (ARM64) or Linux (x86_64)

DDS → PNG conversion is done in-process (BC1/BC3); no external tools required.

## Quick Start

### From a release

Download the archive for your platform, unpack it, and run the binary. CascLib is
bundled in `lib/` and resolved relative to the executable, so no environment
variables are needed:

```bash
tar xzf casc-extractor-<version>-aarch64-apple-darwin.tar.gz
cd casc-extractor-<version>-aarch64-apple-darwin
./casc-extractor inspect archive
```

To put it on your `PATH`, keep the directory intact and symlink the binary:

```bash
sudo mv casc-extractor-<version>-aarch64-apple-darwin /usr/local/opt/casc-extractor
sudo ln -sf /usr/local/opt/casc-extractor/casc-extractor /usr/local/bin/casc-extractor
casc-extractor inspect archive
```

### From source

```bash
# Build CascLib into ./lib first — see lib/README.md
cargo build --release

# Auto-detects the StarCraft installation. Use --install-path to override.
./target/release/casc-extractor <command>
```

The build links `@executable_path/lib` and `@loader_path/../../lib`, so running
from `target/release/` (or via `cargo run`) finds CascLib without
`DYLD_LIBRARY_PATH`/`LD_LIBRARY_PATH`. If you copy the bare binary somewhere else,
copy `lib/` next to it too.

## Commands

### `extract anim` — HD/SD animations

```bash
# Extract raw ANIM files (IDs 0, 1, 2)
casc-extractor extract anim --quality hd4 --ids 0,1,2

# Convert diffuse layer to PNG
casc-extractor extract anim --quality hd4 --ids 0 --convert-to-png

# Convert with team-color mask (see docs/team-color.md for compositing)
casc-extractor extract anim --quality hd4 --ids 0 --convert-to-png --team-color-mask

# Keep raw DDS alongside PNG; export additional texture layers
casc-extractor extract anim --quality hd4 --ids 0 --convert-to-png \
  --save-dds --layers diffuse,normal,specular

# All 999 sprites, hd4 quality, named from mapping file
casc-extractor extract anim --quality hd4 --convert-to-png

# All SD sprites (868 named PNGs from the single mainSD.anim file)
casc-extractor extract anim --quality sd --convert-to-png
```

`extract anim` flags:

| Flag | Description |
|------|-------------|
| `--quality hd4\|hd2\|sd` | Quality level (default: `hd4`) |
| `--ids 0,1,7` | Specific anim IDs to extract |
| `--convert-to-png` | Decode diffuse DDS to PNG + JSON metadata |
| `--team-color-mask` | Also export `_tc.png` binary mask for team-colour compositing |
| `--save-dds` | Keep raw DDS alongside PNG |
| `--layers diffuse,teamcolor,…` | Additional texture layers to export (default: `diffuse`) |
| `--name-map names.json` | JSON map of `{"id": "name"}` for output filenames |

### `extract tileset`

```bash
casc-extractor extract tileset --quality hd4 --convert-to-png
```

### `extract effect`

```bash
casc-extractor extract effect --quality hd4 --convert-to-png
```

### Terrain and tileset extraction

```bash
# Atlas of raw tileset tiles
casc-extractor extract tileset-atlas --tileset jungle --max-tiles 64 --cols 8

# Atlas of assembled megatiles (VX4EX-indexed)
casc-extractor extract tileset-megatile --tileset jungle --max-tiles 64

# Exactly the terrain a given map uses, as an atlas + minimap + JSON
casc-extractor extract map-terrain --scm "/path/to/(8)Char Magma.scm"

# A raw, non-MPQ scenario.chk (e.g. an extracted campaign mission)
casc-extractor extract map-terrain --chk scenario.chk --name Terran01

# Dual-layer variant (diffuse + opaque base)
casc-extractor extract map-terrain-dual --scm "/path/to/map.scm"
```

### Other extraction subcommands

```bash
casc-extractor extract mainmenu --convert-to-png   # main-menu art
casc-extractor extract cmdicons --convert-to-png   # command-card icons
casc-extractor extract dat                         # units.dat, weapons.dat, …
```

### `sounds extract`

```bash
casc-extractor sounds extract
casc-extractor sounds list                        # enumerate audio paths
casc-extractor sounds export-targets              # write built-in target list to JSON
casc-extractor sounds extract --targets my.json  # use custom target list
casc-extractor sounds probe                        # report which targets resolve
```

### `extract raw` — raw file extraction

Extracts one or more files from the CASC archive without any format conversion.
Useful for UI layout files (`.ui.json`), data tables, and any other file type
not covered by the other subcommands.

```bash
# List all .ui.json files in the archive
casc-extractor inspect files --search ".ui.json"

# Extract SC:R UI layout files — requires internet/CDN access (--online)
casc-extractor extract raw --search "statbtn" --locale enUS --output /tmp/scr_ui --online

# Extract a single file by exact path (locale prefix added automatically with --online)
casc-extractor extract raw "Assets\rez\statbtnn.ui.json" --output /tmp --online

# Supply the full locale-prefixed path explicitly
casc-extractor extract raw "locales\enUS\Assets\rez\statbtnn.ui.json" --output /tmp --online

# Extract without --online (only works for files present in the local install package)
casc-extractor extract raw --search "cmdicons" --output /tmp/icons
```

`extract raw` flags:

| Flag | Description |
|------|-------------|
| `FILE_PATH …` | One or more exact CASC archive paths. Mutually exclusive with `--search`. |
| `--search <pattern>` | Extract all files whose path contains this substring (case-insensitive). |
| `--locale <code>` | Locale for the automatic `--online` path prefix, and for narrowing `--search` results that live under `locales\` (default: `enUS`). Locale-neutral paths are never filtered out. |
| `--online` | Open the CDN-backed archive to reach files not present in the local install. |
| `--preserve-path` / `-p` | Write output to the full archive sub-path instead of filename-only. |

#### CDN-only files and the locale prefix

Some SC:R files — notably all `.ui.json` UI layout files — are not included in
the local installation package and can only be fetched from Blizzard's CDN.
These require `--online`.

Inside the CASC archive these files live under a locale prefix:

```
locales\enUS\Assets\rez\statbtnn.ui.json
```

When `--online` is set and a positional path does **not** already start with
`locales\`, the tool prepends `locales\<locale>\` automatically.  So both of
these are equivalent:

```bash
casc-extractor extract raw "Assets\rez\statbtnn.ui.json" --online
casc-extractor extract raw "locales\enUS\Assets\rez\statbtnn.ui.json" --online
```

When using `--search`, the locale only narrows paths that are themselves under
`locales\`, so `--locale enUS` restricts localised hits to the `enUS` tree while
still matching the locale-neutral majority of the archive.

#### Embedded fallbacks

A set of known SC:R UI layout files is compiled directly into the binary.
When a live extraction attempt fails (file not in local install, no `--online`),
the tool checks this embedded table before reporting the file as skipped.
This means the six core command-card layout files are always available offline:

| CASC path | File |
|-----------|------|
| `locales\enUS\Assets\rez\statbtnn.ui.json` | Normal button layout |
| `locales\enUS\Assets\rez\statbtnp.ui.json` | Protoss button layout |
| `locales\enUS\Assets\rez\statbtnt.ui.json` | Terran button layout |
| `locales\enUS\Assets\rez\statbtnz.ui.json` | Zerg button layout |
| `locales\enUS\Assets\rez\statdata.ui.json` | Unit stat data layout |
| `locales\enUS\Assets\rez\statport.ui.json` | Portrait stat layout |

```bash
# Works without --online or a local install:
casc-extractor extract raw --search statbtn --locale enUS --output /tmp/scr_ui
```

Output summary distinguishes live vs embedded extractions:
```
0 extracted, 4 from embedded fallback, 4 skipped (of 8 requested).
```

### `inspect`

```bash
casc-extractor inspect archive          # archive file count and basic info
casc-extractor inspect sprites --max-id 20
casc-extractor inspect files --search cmdicons          # search archive paths
casc-extractor inspect files --search .wpe --sizes      # include file sizes
```

### `config`

```bash
casc-extractor config init              # write default config to casc-config.json
casc-extractor config init --output my-config.json
casc-extractor -c my-config.json extract anim --quality hd4
```

### `validate`

```bash
casc-extractor validate register --file output/main_000.png --suite regression.json
casc-extractor validate run --dir output/ --suite regression.json
```

## Global Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--install-path <path>` | | Override auto-detected StarCraft installation |
| `--output <dir>` | | Output directory (default: `extracted`; overrides config) |
| `--config <path>` | `-c` | Path to a JSON config file |
| `--verbose` | `-v` | Enable debug logging |
| `--validate-only` | | Dry run: confirm the archive opens and list files, then exit without writing. Applies to every subcommand. |

## Configuration File

Generate a template:

```bash
casc-extractor config init
```

```json
{
  "quality_settings": {
    "format_filter": "All",
    "png_compression_level": 6
  },
  "output_settings": {
    "output_directory": "output",
    "overwrite_behavior": "IfNewer",
    "metadata_options": { "generate_json": true },
    "unity_settings": { "pixels_per_unit": 100.0 }
  },
  "filter_settings": {
    "max_files": null,
    "include_patterns": null,
    "exclude_patterns": null
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `quality_settings.format_filter` | `All`\|`Png`\|`Images` | When set to `Png`, implies `--convert-to-png` on all extract commands |
| `quality_settings.png_compression_level` | 0–9 | PNG compression (default: 6) |
| `output_settings.overwrite_behavior` | enum | `Always` `Never` `IfNewer` `Backup` `Prompt` |
| `output_settings.metadata_options.generate_json` | bool | Write Unity-compatible JSON per sprite |
| `output_settings.unity_settings.pixels_per_unit` | float | Pixels-per-unit in JSON metadata (default: 100) |
| `filter_settings.max_files` | int or null | Cap on files processed per run |
| `filter_settings.include_patterns` | string[] or null | Regex allow-list on CASC paths |
| `filter_settings.exclude_patterns` | string[] or null | Regex deny-list on CASC paths |

## Team Colour Compositing

SC:R HD sprites store team colour as a binary DXT1 mask (layer 2, named
`teamcolor`). The correct compositing formula is:

```
output.rgb = diffuse.rgb * (player_color.rgb / 255)   — for masked pixels
output      = diffuse                                  — for unmasked pixels
```

Standard SC1 player colours: Red `#F40404`, Blue `#0C48CC`, Teal `#2CB494`,
Purple `#88409C`, Orange `#F88C14`, Yellow `#FCFC38`.

Full algorithm, layer layout, and Python reference implementation:
**[docs/team-color.md](docs/team-color.md)**

## Architecture

```
src/
├── main.rs             — CLI definition (clap) + dispatch only
├── commands/           — subcommand handlers, grouped by subject
│   ├── support.rs      — path safety, overwrite policy, config, archive opening
│   ├── sprites.rs      — extract anim / tileset / effect
│   ├── terrain.rs      — tileset atlases, map-terrain, VR4 + CHK parsing
│   ├── assets.rs       — cmdicons, mainmenu, dat, raw
│   ├── sounds.rs       — sounds extract / list / probe / export-targets
│   ├── inspect.rs      — inspect sprites / archive / files
│   └── maintenance.rs  — config init, validate register / run
├── lib.rs              — public API
├── anim/               — HD ANIM parser (v0x0202/0x0204) + SD ANIM (v0x0101)
│   └── hd_parser.rs    — HdAnimFile, layer extraction, team-colour helpers
├── casc/               — CascLib FFI, archive discovery, file enumeration
├── config/             — ExtractionConfig (serde JSON) + FormatFilterOption
├── dds_converter.rs    — DDS → PNG (BC1/BC3) via ddsfile + texture2ddecoder
├── grp/                — GRP (SD) parser, RLE decoder, spritesheet builder
├── sprite/export.rs    — ExportConfig, export_anim pipeline
├── mapping/            — YAML sprite-name → CASC-path loader
├── palette/            — 256-colour game palette
├── progress/           — indicatif progress reporter
└── validation/         — Byte comparison, regression suite
tests/
└── baselines/          — committed SHA256 suite guarding pixel output
docs/
└── team-color.md       — HD team colour layer structure and compositing algorithm
```

Pixel output is pinned by `tests/baselines/validation-suite.json`. If a change is
meant to alter output, re-register the affected files:

```bash
casc-extractor validate register <file> --suite tests/baselines/validation-suite.json
```

## Legal Notice

You must own a legal copy of the game to use this tool. Extracted assets are
property of Blizzard Entertainment and may only be used for personal or
educational purposes.

StarCraft® is a registered trademark of Blizzard Entertainment, Inc. This
project is not affiliated with, endorsed by, or sponsored by Blizzard
Entertainment, Inc.

## Acknowledgments

- [CascLib](https://github.com/ladislav-zezula/CascLib) by Ladislav Zezula
- StarCraft sprite format research by the modding community
- Team colour shader reverse-engineering: [neivv/mtl](https://github.com/neivv/mtl)

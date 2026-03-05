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
- ImageMagick (`magick`) for DDS → PNG conversion (optional but recommended)

## Quick Start

```bash
# Build
cargo build --release

# Auto-detects StarCraft installation. Use --install-path to override.
DYLD_LIBRARY_PATH=lib ./target/release/casc-extractor <command>
```

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

### `extract organized` — mapping-driven extraction

Extracts sprites from a YAML mapping file that maps unit names to CASC paths,
producing a categorised directory tree.

```bash
casc-extractor extract organized --mapping mappings/starcraft-remastered.yaml \
  --quality sd --convert-to-png

# HD with team-color mask and extra layers
casc-extractor extract organized --quality hd4 --convert-to-png \
  --team-color-mask --layers diffuse,normal
```

Output structure:
```
output/
├── terran/units/       # marine, ghost, firebat, …
├── terran/buildings/
├── protoss/units/
├── protoss/buildings/
├── zerg/units/
├── zerg/buildings/
├── effects/
├── neutral/
└── ui/
```

### `sounds extract`

```bash
casc-extractor sounds extract
casc-extractor sounds list                        # enumerate audio paths
casc-extractor sounds export-targets              # write built-in target list to JSON
casc-extractor sounds extract --targets my.json  # use custom target list
```

### `inspect`

```bash
casc-extractor inspect archive          # archive file count and basic info
casc-extractor inspect sprites --max-id 20
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
| `--output <dir>` | | Output directory (default: `output`; overrides config) |
| `--config <path>` | `-c` | Path to a JSON config file |
| `--verbose` | `-v` | Enable debug logging |
| `--validate-only` | | Open archive and list files without writing output |

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
├── main.rs             — unified CLI (clap), all subcommand handlers
├── lib.rs              — public API
├── anim/               — HD ANIM parser (v0x0202/0x0204) + SD ANIM (v0x0101)
│   └── hd_parser.rs    — HdAnimFile, layer extraction, team-colour helpers
├── casc/               — CascLib FFI, archive discovery, file enumeration
├── config/             — ExtractionConfig (serde JSON, all fields wired)
├── dds_converter.rs    — DDS → PNG via ImageMagick + ddsfile fallback
├── grp/                — GRP (SD) parser, RLE decoder, spritesheet builder
├── sprite/export.rs    — ExportConfig, export_anim pipeline
├── mapping/            — YAML sprite-name → CASC-path loader
├── palette/            — 256-colour game palette
├── filter/             — Regex include/exclude + format filter
├── progress/           — indicatif progress reporter
└── validation/         — Byte comparison, regression suite
docs/
└── team-color.md       — HD team colour layer structure and compositing algorithm
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

# casc-extractor

Extract sprites and audio from StarCraft: Remastered CASC archives.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Supported Games

**StarCraft: Remastered** — fully tested (133 sprites extracted)

Other CASC-based games are compatible but untested: Warcraft III: Reforged, Heroes of the Storm, World of Warcraft, Diablo III, Overwatch.

*Other games may require format-specific parsers.*

## Extraction Results

StarCraft: Remastered includes:
- 44 units (Terran, Protoss, Zerg)
- 46 buildings (all structures)
- 33 effects (projectiles, explosions, fire, smoke, blood)
- 7 neutral (critters, resources)
- 3 UI elements (wireframes)

### HD Content Quality Levels

StarCraft: Remastered provides three quality levels:

#### 4x HD (Ultra — 4K)
- **CASC path**: `anim/`, `tileset/`, `effect/` (no prefix)
- **Animations**: ~4.5 MB per sprite (999 sprites available)
- **Tilesets**: ~55 MB per tileset (8 tilesets)

#### 2x HD
- **CASC path**: `HD2/anim/`, `HD2/tileset/`, `HD2/effect/`
- **Animations**: ~1.1 MB per sprite
- **Tilesets**: ~14 MB per tileset

#### SD (Original)
- **CASC path**: `SD/mainSD.anim` (single 38 MB file, all sprites)
- **Format**: Paletted 256-color GRP graphics

**Total HD content**: ~5.2 GB in the game's `Data/data/` folder.

## Requirements

- Rust 1.70+
- StarCraft: Remastered (or another CASC-based Blizzard game)
- macOS (ARM64) or Linux (x86_64)
- CascLib native library — must be built from source. See [`lib/README.md`](lib/README.md).

## Quick Start

```bash
# Build
cargo build --release

# Auto-detects your StarCraft installation.
# Override with --install-path /path/to/StarCraft if needed.
DYLD_LIBRARY_PATH=lib ./target/release/casc-extractor <command>
```

### Extract HD sprites (IDs 0, 1, 2)

```bash
casc-extractor extract anim --quality hd4 --ids 0,1,2
```

### Convert to PNG

```bash
casc-extractor extract anim --quality hd4 --ids 0 --convert-to-png
```

### Extract all SD sprites

```bash
casc-extractor extract anim --quality sd
```

### Extract tilesets

```bash
casc-extractor extract tileset
```

### Extract organized (using sprite mapping)

```bash
casc-extractor extract organized
```

### Extract sounds

```bash
casc-extractor sounds extract
```

### Inspect archive

```bash
casc-extractor inspect archive
```

### Inspect which sprite IDs exist

```bash
casc-extractor inspect sprites --max-id 20
```

## Global Flags

These flags apply to all subcommands:

| Flag | Short | Description |
|------|-------|-------------|
| `--install-path <path>` | | Override auto-detected StarCraft installation directory |
| `--output <dir>` | | Output directory (default: `output`; overrides config) |
| `--config <path>` | `-c` | Path to a JSON config file |
| `--verbose` | `-v` | Enable debug logging |

## Configuration

Generate a template config file:

```bash
casc-extractor config init
# Writes casc-config.json in the current directory.
# Use --output to choose a different path.
casc-extractor config init --output my-config.json
```

Use the config file with any command:

```bash
casc-extractor -c my-config.json extract anim --quality hd4
```

### Key config fields

```json
{
  "output_settings": {
    "output_directory": "output",
    "overwrite_behavior": "IfNewer",
    "metadata_options": {
      "generate_json": true
    },
    "unity_settings": {
      "pixels_per_unit": 100.0
    }
  },
  "quality_settings": {
    "png_compression_level": 6
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
| `output_settings.output_directory` | string | Base output directory |
| `output_settings.overwrite_behavior` | enum | `Always`, `Never`, `IfNewer`, `Backup`, `Prompt` |
| `output_settings.metadata_options.generate_json` | bool | Write JSON metadata alongside extracted files |
| `output_settings.unity_settings.pixels_per_unit` | float | Pixels per unit for Unity metadata (default: 100) |
| `quality_settings.png_compression_level` | int 0–9 | PNG compression level (default: 6) |
| `filter_settings.max_files` | int or null | Cap on number of files to process |
| `filter_settings.include_patterns` | string[] or null | Regex patterns — only matching CASC paths are extracted |
| `filter_settings.exclude_patterns` | string[] or null | Regex patterns — matching CASC paths are skipped |

## Advanced Features

### Name map for `extract anim`

Supply a JSON file mapping anim IDs to unit names. Each extracted file will
receive an additional copy named after the unit.

```bash
casc-extractor extract anim --quality hd4 --name-map names.json
```

`names.json` format:

```json
{
  "0": "marine",
  "1": "ghost",
  "7": "zealot"
}
```

### PNG conversion flags

```bash
# Extract diffuse layer as PNG
casc-extractor extract anim --quality hd4 --ids 0 --convert-to-png

# Also write team-color mask alongside diffuse PNG
casc-extractor extract anim --quality hd4 --ids 0 --convert-to-png --team-color-mask
```

### Sound discovery fallback

`sounds extract` first tries a curated list of known CASC paths for each
sound. If none succeed, it falls back to dynamic discovery: it scans the full
archive file listing for a `.wav`/`.ogg` whose path contains all keywords from
the output filename. Sounds are saved as `.ogg` files.

```bash
casc-extractor sounds extract
casc-extractor sounds list   # probe paths and enumerate Zerg/UI audio
```

### SD extraction

SD quality extracts `SD/mainSD.anim` — a single 38 MB file containing all
sprites in GRP format. Pass `--convert-to-png` to render a spritesheet PNG.

```bash
casc-extractor extract anim --quality sd --convert-to-png
```

### Organized extraction

Extracts sprites according to a YAML mapping file that maps category paths to
CASC paths, placing output in a categorized directory tree. Defaults to
`mappings/starcraft-remastered.yaml`.

```bash
casc-extractor extract organized --mapping mappings/starcraft-remastered.yaml
```

Output structure:

```
output/
├── terran/units/       # Marine, Firebat, Ghost, …
├── terran/buildings/   # Command Center, Barracks, …
├── protoss/units/
├── protoss/buildings/
├── zerg/units/
├── zerg/buildings/
├── effects/
├── neutral/
└── ui/
```

## Architecture

```
src/
├── main.rs             — unified CLI (clap), all subcommand handlers
├── lib.rs              — public API: export_anim, CascStorage, ExportConfig
├── anim/               — HD ANIM format parser and frame export
├── casc/               — CascLib FFI, archive discovery, file enumeration
├── config/             — ExtractionConfig (serde JSON)
├── grp/                — GRP (SD sprite) parser and RLE decoder
├── mapping.rs          — YAML sprite mapping loader
├── palette.rs          — 256-color game palette
├── sprite/             — PNG spritesheet builder and export pipeline
├── filter/             — Include/exclude regex filtering
├── progress/           — Progress reporter
└── validation/         — Byte comparison, visual validation helpers
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

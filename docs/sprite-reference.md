# Sprite Reference

What the extractor can pull out of StarCraft: Remastered, and how sprites are named.

## Scale

Sprites are addressed by ANIM id, not by a hand-curated list:

| Source | Count | Notes |
|--------|-------|-------|
| HD 4x | up to 1000 | `anim/main_000.anim` … `main_999.anim`; ~872 ids exist in this build |
| HD 2x | up to 1000 | Same ids under `HD2/` |
| SD | 868 | All inside the single `SD/mainSD.anim` (38 MB, ANIM v0x0101) |

Enumerate what your archive actually contains:

```bash
casc-extractor inspect sprites --max-id 1000
```

## Naming

`mappings/starcraft-remastered.yaml` maps readable names to archive paths. It has
~1040 entries, keyed by name and valued by CASC path:

```yaml
animations/terran/battlecruiser: anim/main_218.anim
animations/zerg/guardian: anim/main_025.anim
```

`extract anim` inverts this to name its output, so you get `battlecruiser.anim`
rather than `main_218.anim`. Pass `--raw` for numbered filenames, or `--name-map
my.json` to override with your own `{"218": "some/name"}` table.

Most entries are `animations/<race>/<unit>`, but the file also covers effects and
scenery — flames, blood variants, selection circles, vespene smoke, mineral fields,
psi storm parts, explosions — plus `tilesets/`, `ui/` and `data/` prefixes for the
non-sprite commands.

## Output per sprite

With `--convert-to-png`:

| File | Contents |
|------|----------|
| `{name}.png` | Diffuse layer as a sprite sheet |
| `{name}.json` | Frame metadata (see below) |
| `{name}_tc.png` | Team-colour mask, only with `--team-color-mask` |
| `{name}_{layer}.png` | One per extra `--layers` entry |
| `{name}.dds` | Raw diffuse DDS, only with `--save-dds` |

Without `--convert-to-png` you get the raw `{name}.anim` instead.

The JSON schema is:

```json
{
  "name": "main_025",
  "frameCount": 119,
  "grpWidth": 421,
  "grpHeight": 420,
  "textureWidth": 4072,
  "textureHeight": 1560,
  "pixelsPerUnit": 100.0,
  "frames": [
    {"index": 0, "x": 3208, "y": 1408, "width": 268, "height": 140,
     "offsetX": 79, "offsetY": 101}
  ]
}
```

Frames carry `offsetX`/`offsetY` because ANIM frames are trimmed — the offset is
where the frame sits relative to the logical sprite bounds, and ignoring it makes
animations jitter.

## Available layers

HD sprites carry up to seven layers; `--layers` selects which to export.

| Layer | Format | Notes |
|-------|--------|-------|
| `diffuse` | DXT5 | Base texture (the default) |
| `teamcolor` | DXT1 | Binary mask; see [team-color.md](team-color.md) |
| `emissive` | DXT5 | Often empty |
| `normal` | DXT5 | Normal map |
| `specular` | DXT1 | Specular mask |
| `ao` | DXT5 | Spelled `ao_depth` in the file header |

Roughly half of all sprites have a non-empty `teamcolor` layer; the rest are neutral
units or effects.

## Extending

Add a line to `mappings/starcraft-remastered.yaml`:

```yaml
animations/terran/my_unit: anim/main_042.anim
```

Find the id first with `casc-extractor inspect sprites`, or search the archive
directly with `casc-extractor inspect files --search marine`.

See [Getting Started](getting-started.md) for the extraction commands.

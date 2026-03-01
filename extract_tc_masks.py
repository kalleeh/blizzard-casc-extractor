#!/usr/bin/env python3
"""
extract_tc_masks.py
-------------------
Re-extracts sprites with --team-color-mask to produce:
  - <sprite>_tc.png   : grayscale team-colour mask alongside each diffuse PNG
  - <sprite>.png       : diffuse with team-colour pixels hue-stripped (neutral gray)

Steps:
  1. Parse starcraft-remastered.yaml -> sprite_basename -> anim_id mapping
  2. Scan Assets/Art/Sprites/StarCraft_HD/** in the Unity project for existing *.png files
  3. Run casc-extractor extract anim --team-color-mask for all matched anim IDs in one pass
  4. Copy _tc.png + updated diffuse .png into the Unity sprite directories

Usage:
  python3 extract_tc_masks.py --unity-project /path/to/starcraft-reimagined
  python3 extract_tc_masks.py  # auto-detects sibling Unity project
"""
import argparse, os, re, shutil, subprocess, sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent

# ---------------------------------------------------------------------------
# Resolve Unity project root
# ---------------------------------------------------------------------------
parser = argparse.ArgumentParser(description="Re-extract TC masks into a Unity project")
parser.add_argument(
    "--unity-project",
    type=Path,
    default=None,
    help="Path to the starcraft-reimagined Unity project root (auto-detected if omitted)",
)
args = parser.parse_args()

if args.unity_project:
    UNITY_ROOT = args.unity_project.resolve()
else:
    # Default: sibling directory named starcraft-reimagined
    candidate = SCRIPT_DIR.parent / "starcraft-reimagined"
    if not candidate.exists():
        # Also try the Unity/ subfolder pattern
        candidate = Path.home() / "Unity" / "starcraft-reimagined"
    if not candidate.exists():
        print(
            "ERROR: Cannot find Unity project. Pass --unity-project <path>",
            file=sys.stderr,
        )
        sys.exit(1)
    UNITY_ROOT = candidate.resolve()

UNITY_SPRITES = UNITY_ROOT / "Assets/Art/Sprites/StarCraft_HD"
if not UNITY_SPRITES.exists():
    print(f"ERROR: Unity sprites directory not found: {UNITY_SPRITES}", file=sys.stderr)
    sys.exit(1)

YAML_PATH   = SCRIPT_DIR / "mappings/starcraft-remastered.yaml"
EXTRACT_OUT = SCRIPT_DIR / "output/tc_extraction"
binary      = SCRIPT_DIR / "target/release/casc-extractor"
lib_dir     = str(SCRIPT_DIR / "lib")

print(f"Unity project : {UNITY_ROOT}")
print(f"Sprites dir   : {UNITY_SPRITES}")

# ---------------------------------------------------------------------------
# 1. Parse the YAML mapping:  sprite_basename -> anim_id
#    Only care about lines that match:
#      animations/<path>: anim/main_<id>.anim
# ---------------------------------------------------------------------------
anim_id_for = {}   # e.g. "marine" -> 239
pattern = re.compile(r'^animations/([^\s:]+)\s*:\s*anim/main_(\d+)\.anim')

with open(YAML_PATH) as f:
    for line in f:
        m = pattern.match(line.strip())
        if m:
            key, anim_id = m.group(1), int(m.group(2))
            basename = key.split("/")[-1]   # "terran/marine" -> "marine"
            anim_id_for[basename] = anim_id

print(f"\nParsed {len(anim_id_for)} unique sprite names from YAML")

# ---------------------------------------------------------------------------
# 2. Walk Unity sprite directory, match PNGs to anim IDs
# ---------------------------------------------------------------------------
to_extract = []   # (anim_id, dest_dir, basename)
missing    = []

for png in sorted(UNITY_SPRITES.rglob("*.png")):
    if "_tc" in png.name:
        continue            # skip already-generated masks
    basename = png.stem     # "marine.png" -> "marine"
    if basename in anim_id_for:
        to_extract.append((anim_id_for[basename], png.parent, basename))
    else:
        missing.append(png.relative_to(UNITY_SPRITES))

print(f"\nMatched {len(to_extract)} sprites to anim IDs")
if missing:
    print(f"No YAML match for {len(missing)} sprites (overlays/effects, skipping):")
    for p in missing[:10]:
        print(f"  {p}")
    if len(missing) > 10:
        print(f"  ... and {len(missing)-10} more")

if not to_extract:
    print("Nothing to extract — exiting.")
    sys.exit(0)

# ---------------------------------------------------------------------------
# 3. Run the extractor
# ---------------------------------------------------------------------------
anim_ids = sorted(set(a for a, _, _ in to_extract))
ids_str  = ",".join(str(i) for i in anim_ids)

EXTRACT_OUT.mkdir(parents=True, exist_ok=True)

cmd = [
    str(binary),
    "--output", str(EXTRACT_OUT),
    "extract", "anim",
    "--convert-to-png",
    "--team-color-mask",
    "--ids", ids_str,
]

print(f"\nRunning extractor for {len(anim_ids)} anim IDs…")
print(f"  binary: {binary}")
print(f"  DYLD_LIBRARY_PATH: {lib_dir}")
print("  (this will take several minutes)\n")

env = os.environ.copy()
env["DYLD_LIBRARY_PATH"] = lib_dir

result = subprocess.run(cmd, cwd=SCRIPT_DIR, env=env)
if result.returncode != 0:
    print(f"\nExtractor exited with code {result.returncode} — check output above.")
    sys.exit(result.returncode)

# ---------------------------------------------------------------------------
# 4. Copy outputs into Unity sprite directories
# ---------------------------------------------------------------------------
copied_tc      = 0
copied_diffuse = 0
missing_output = []

for anim_id, dest_dir, basename in to_extract:
    # New unified CLI writes into a subdirectory named after the anim file
    src_dir = EXTRACT_OUT / f"main_{anim_id:03d}"
    src_png = src_dir / f"main_{anim_id:03d}.png"
    src_tc  = src_dir / f"main_{anim_id:03d}_tc.png"
    # Fallback: flat layout (old extract_hd behaviour)
    if not src_png.exists():
        src_png = EXTRACT_OUT / f"main_{anim_id:03d}.png"
        src_tc  = EXTRACT_OUT / f"main_{anim_id:03d}_tc.png"

    dst_png = dest_dir / f"{basename}.png"
    dst_tc  = dest_dir / f"{basename}_tc.png"

    if src_png.exists():
        shutil.copy2(src_png, dst_png)
        copied_diffuse += 1
    else:
        missing_output.append(f"main_{anim_id:03d}.png ({basename})")

    if src_tc.exists():
        shutil.copy2(src_tc, dst_tc)
        copied_tc += 1

print(f"\n{'='*60}")
print(f"Done!")
print(f"  Diffuse PNGs updated : {copied_diffuse}")
print(f"  TC masks written     : {copied_tc}")
if missing_output:
    print(f"  Missing outputs      : {len(missing_output)}")
    for m in missing_output[:5]:
        print(f"    {m}")
print(f"{'='*60}")
print(f"\nNext steps:")
print(f"  1. Switch to Unity — the sprite reimport will trigger automatically")
print(f"     (HDSpriteImporter.OnPreprocessTexture runs on every .png change)")

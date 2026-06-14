#!/usr/bin/env python3
"""Generate image_map.json from the YAML mapping file."""
import json
import os
import re

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(SCRIPT_DIR)
YAML_PATH = os.path.join(ROOT, "mappings", "starcraft-remastered.yaml")
OUTPUT_PATH = os.path.join(ROOT, "extracted", "dat", "image_map.json")

PATTERN = re.compile(r"^animations/(.+?):\s*anim/main_(\d+)\.anim")

image_map = {}
with open(YAML_PATH) as f:
    for line in f:
        m = PATTERN.match(line)
        if m:
            sprite_name, image_id = m.group(1), m.group(2)
            image_map[image_id] = sprite_name

sorted_map = dict(sorted(image_map.items(), key=lambda kv: int(kv[0])))
os.makedirs(os.path.dirname(OUTPUT_PATH), exist_ok=True)
with open(OUTPUT_PATH, "w") as f:
    json.dump(sorted_map, f, indent=2)

print(f"Wrote {len(image_map)} entries to {OUTPUT_PATH}")

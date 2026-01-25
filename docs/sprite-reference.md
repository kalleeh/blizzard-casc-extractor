# Sprite Reference

Complete reference of extractable sprites from StarCraft: Remastered.

## Overview

The extractor can extract **133 sprites** organized into:
- **44 units** (all playable units)
- **46 buildings** (all structures)
- **33 effects** (projectiles, explosions, etc.)
- **7 neutral** (critters, resources)
- **3 UI** (wireframes)

## Units

### Terran (13)
- Marine
- Firebat
- Ghost
- Vulture
- Goliath
- Siege Tank
- SCV
- Wraith
- Dropship
- Battlecruiser
- Science Vessel
- Medic
- Valkyrie

### Protoss (15)
- Probe
- Zealot
- Dragoon
- High Templar
- Dark Templar
- Archon
- Dark Archon
- Shuttle
- Scout
- Corsair
- Carrier
- Arbiter
- Reaver
- Observer
- Interceptor

### Zerg (16)
- Larva
- Egg
- Cocoon
- Drone
- Zergling
- Hydralisk
- Ultralisk
- Mutalisk
- Guardian
- Devourer
- Scourge
- Queen
- Defiler
- Overlord
- Lurker
- Broodling

## Buildings

### Terran (15)
Command Center, Supply Depot, Refinery, Barracks, Academy, Factory, Starport, Control Tower, Science Facility, Missile Turret, Engineering Bay, Armory, Bunker, Physics Lab, Machine Shop

### Protoss (16)
Nexus, Pylon, Assimilator, Gateway, Forge, Cybernetics Core, Photon Cannon, Citadel, Robotics Facility, Stargate, Templar Archives, Observatory, Arbiter Tribunal, Robotics Support Bay, Fleet Beacon, Shield Battery

### Zerg (15)
Hatchery, Lair, Hive, Creep Colony, Sunken Colony, Spore Colony, Extractor, Spawning Pool, Evolution Chamber, Hydralisk Den, Spire, Queens Nest, Nydus Canal, Ultralisk Cavern, Defiler Mound

## Effects

### Projectiles (18)
Marine Bullet, Gauss Rifle, Missile, Yamato Cannon, Psi Storm, Psi Blade, Scarab, Photon Blast, Phase Disruptor, Needle Spine, Acid Spore, Glave Wurm, Seeker Spore, Subterranean Spines, Corrosive Acid, Particle Beam, Grenade, Fragmentation Grenade

### Explosions (3)
EMP, Nuclear Hit, Nuclear Beam

### Environmental (10)
Fire (Flame, Building Large, Building Small), Smoke (Building, Green), Blood (Terran Small/Large, Zerg Small/Large), Dust, Shield Hit

## Neutral

### Resources (1)
Vespene Geyser

### Critters (6)
Rhynadon, Bengalaas, Scantid, Kakaru, Ragnasaur, Ursadon

## UI

### Wireframes (3)
Terran, Protoss, Zerg unit outlines

## Output Format

Each sprite is extracted as:
- `{name}.png` - Sprite sheet (17 frames per row)
- `{name}.json` - Unity metadata
- `{name}.txt` - Frame information

## File Locations

```
extracted/
├── terran/units/
├── terran/buildings/
├── protoss/units/
├── protoss/buildings/
├── zerg/units/
├── zerg/buildings/
├── effects/projectiles/
├── effects/explosions/
├── effects/fire/
├── effects/smoke/
├── effects/blood/
├── effects/dust/
├── effects/shields/
├── neutral/buildings/
├── neutral/critters/
└── ui/wireframes/
```

## Extending

To add more sprites, edit `mappings/starcraft-remastered.yaml`:

```yaml
category/subcategory/name: archive\path\to\file.grp
```

See [Getting Started](getting-started.md) for configuration details.

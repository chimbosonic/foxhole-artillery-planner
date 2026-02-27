# Artillery Calculation Notes

Reference documentation for the math and data behind the Foxhole Artillery Planner.

## Coordinate System

- **X axis** = east (positive), **Y axis** = south (positive) — standard screen coordinates
- Map images are 2048×1776 pixels, representing a Foxhole hex region
- Positions are stored and calculated in **meters** (converted from/to image pixels via scale factors in `crates/shared/src/grid.rs`)

## Azimuth

Compass bearing from gun to target in degrees [0°, 360°), clockwise from north.

```
azimuth = atan2(dx, -dy)   where dx = target.x - gun.x, dy = target.y - gun.y
```

- North = 0°, East = 90°, South = 180°, West = 270°
- The `-dy` negation converts screen-Y (south-positive) to compass-Y (north-positive)

**Source**: `crates/shared/src/calc.rs` — `azimuth()`

## Distance

Euclidean distance between gun and target in meters.

```
distance = √(dx² + dy²)
```

**Source**: `crates/shared/src/calc.rs` — `distance()`

## Accuracy Radius

Linear interpolation between weapon's min and max accuracy values based on distance:

```
t = clamp((distance - min_range) / (max_range - min_range), 0, 1)
accuracy = acc_radius[0] + t × (acc_radius[1] - acc_radius[0])
```

- `acc_radius[0]` = accuracy at minimum range (tighter)
- `acc_radius[1]` = accuracy at maximum range (wider)

**Source**: `crates/shared/src/calc.rs` — `accuracy_radius()`

## Wind Drift

Wind drift varies per weapon and scales linearly with firing distance. Same interpolation pattern as accuracy.

### Base drift at range

```
t = clamp((distance - min_range) / (max_range - min_range), 0, 1)
base_drift = wind_drift[0] + t × (wind_drift[1] - wind_drift[0])
```

### Actual drift (scaled by wind strength)

```
drift_meters = base_drift × (wind_strength / 5.0)
```

Wind strength ranges from 0 to 5 (in-game levels observable on flags/wind socks).

### Wind direction

- Wind direction input is **meteorological**: the compass direction the wind blows **FROM**
- Shells are pushed in the **opposite** direction (where wind blows TO)
- Conversion: `push_direction = (wind_from_direction + 180°) mod 360°`

### Wind offset vector

```
dx_wind = sin(push_direction_radians) × drift_meters
dy_wind = -cos(push_direction_radians) × drift_meters
```

### Wind compensation

To compensate, the aim point is shifted **against** the wind:

```
compensated_target.x = target.x - dx_wind
compensated_target.y = target.y - dy_wind
```

Then azimuth and distance are recalculated to the compensated target.

**Source**: `crates/shared/src/calc.rs` — `wind_drift_at_range()`, `wind_offset()`, `firing_solution()`

## Per-Weapon Wind Drift Values

Data sourced from the [ForsakenNGS/foxhole-spotter](https://github.com/ForsakenNGS/foxhole-spotter) community calculator.

| Ammo Class | Weapons | Wind Drift Min (m) | Wind Drift Max (m) |
|---|---|---|---|
| Mortar | Cremari Mortar | 10 | 40 |
| 120mm | Koronides, Huber Lariat, AC-b Trident, Conquerer/Titan-120mm, Blacksteele/Callahan-120mm | 10 | 30 |
| 150mm | Thunderbolt, Titan-150mm, Callahan-150mm, Huber Exalt, Sarissa, Flood Mk. IX | 15 | 40 |
| 300mm | Storm Cannon, Tempest Cannon RA-2 | 20 | 50 |
| Rockets | Retiarius, Hades' Net, Deioneus, Skycaller, Wasp Nest, King Jester | 15 | 40 |

Rocket values are estimated (same as 150mm class) — the community calculator does not have separate data for rockets.

## References

- [Foxhole Wiki — Artillery](https://foxhole.wiki.gg/wiki/Artillery) — weapon stats, wind mechanics description
- [Foxhole Wiki — Wind](https://foxhole.fandom.com/wiki/Wind) — wind mechanics overview
- [ForsakenNGS/foxhole-spotter](https://github.com/ForsakenNGS/foxhole-spotter) (`js/arty.js`) — per-weapon wind drift values and compensation formula
- [artillery-strike.netlify.app](https://artillery-strike.netlify.app/) — recommended community calculator (linked from wiki)
- [geosheehan/web-artillery-strike](https://github.com/geosheehan/web-artillery-strike) — source for artillery-strike calculator

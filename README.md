# 3D Canon — Wind & Warfare

A native Rust / Bevy artillery game for **two players sharing one computer**.
Aim across a low-poly island, account for the wind, and blast the other cannon.
Every explosion carves a crater into the actual collision surface.

## Run

Install [Rust](https://rustup.rs/), then run from this folder:

```sh
cargo run --locked
```

The first build compiles Bevy and can take several minutes. Subsequent builds are much faster.
Bevy 0.17.3 is selected for compatibility with the installed Rust 1.92 toolchain;
the package's minimum Rust version is 1.88. All models, terrain, effects, and sounds
are generated in code. No asset downloads are needed at runtime.

For a repeatable map and wind sequence:

```sh
cargo run --locked -- --seed=42
```

For an optimized build:

```sh
cargo run --release --locked
```

### Native prerequisites

- **macOS:** Xcode Command Line Tools (`xcode-select --install`) and a Metal-capable GPU.
- **Windows:** Rust's MSVC toolchain and Visual Studio C++ Build Tools.
- **Linux (Ubuntu/Debian):** build tools, a working Vulkan driver, and window/audio development libraries:
  `sudo apt install build-essential pkg-config libasound2-dev libudev-dev libx11-dev libxkbcommon-dev libwayland-dev libvulkan1`.

## How to play

1. Red goes first. Press **Enter** or click **Ready** to take control.
2. Adjust aim, elevation, and power. Press **Space** or click **Fire**.
3. Watch the shell fly and the ground deform. The tracer records the last shot;
   the short team-colored guide shows barrel direction, not a predicted trajectory.
4. Pass the keyboard to the next player when the handoff banner appears.
5. Destroy the opposing cannon to win. Press **R** for a new battlefield.

| Control | Action |
| --- | --- |
| A / D | Rotate aim left / right, relative to the behind-cannon view |
| W / S | Raise / lower elevation (5–85°) |
| Q / E | Decrease / increase power (15–52 m/s) |
| Hold Shift | Fine aiming and power adjustment |
| Enter / Space | Ready during handoff; fire while aiming |
| Right mouse drag | Orbit the aiming camera |
| Mouse wheel | Zoom the aiming camera |
| C | Toggle overview |
| Esc | Pause / resume |
| M | Toggle firing / impact sound |
| R | Start a fresh randomized match |
| Shift + R | Restart the same map and wind sequence |

The HUD displays both health totals, aiming values, seed, round, and wind.
Compass bearings use **0° north (−Z), 90° east (+X)**. Wind is shown as the direction
the air travels **toward**, not where it comes from. The gold arrow above the island
shows the same direction in world space.

### Rules

- Both cannons start with 100 HP. A direct hit destroys a cannon.
- Explosions have an 8 m radius; splash damage falls off linearly with distance.
- Your own cannon can take damage from your shots.
- Each round consists of Red's turn and Blue's turn. They share a wind vector for
  fairness; the next round gets a new vector. Wind stays constant during flight.
- Cannons cannot move horizontally. After an impact they settle down onto the
  lowered terrain. Falling itself does not cause damage.
- Leaving the battlefield or exceeding 18 seconds of flight ends the shot as a miss.
- Terrain deformation stops at bedrock. Craters persist until the next match.
- Simultaneous destruction is treated as a draw.

**A useful first shot:** start around 45° elevation and 32 m/s power, with the
default bearing toward the opponent. Observe the last-shot trace, then correct
for distance, height, and crosswind. Press C to inspect the whole battlefield.

## Architecture

| Module | Responsibility |
| --- | --- |
| `src/main.rs` | App setup and system scheduling |
| `src/game.rs` | Match state, input, fixed simulation, damage, turn handoffs |
| `src/physics.rs` | Analytic gravity/drag integration, swept sphere collision, splash damage |
| `src/terrain.rs` | Seeded landscapes, triangle collision, crater deformation, mesh generation |
| `src/visuals.rs` | Cannon models, lighting, tracer, particles, visual synchronization |
| `src/camera.rs` | Aiming, follow, impact, overview cameras, shake |
| `src/ui.rs` | HUD, health bars, buttons, help, pause display |
| `src/sound.rs` | Procedural PCM firing and explosion sounds |

Simulation runs at 120 Hz independently of display rate. Shell movement integrates
linear drag toward the wind velocity plus gravity. Each step checks the swept
segment against terrain triangles and cannon bounding spheres, resolving the
earliest contact once. Terrain uses exactly the same triangle heights for rendering,
collision, and cannon support.

This first version uses an 80×80-cell heightfield and rebuilds its mesh after each
impact. It supports bowls and trenches, but not caves or overhangs. Cannon support
and collisions are intentionally simplified rather than full rigid-body physics.
Gameplay constants live near the top of `physics.rs` and `terrain.rs`; aiming limits
and initial shot settings are in `game.rs`.

## Development checks

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

Run an automated graphics smoke test (requires a desktop display):

```sh
cargo run --locked -- --seed=42 --smoke-test
```

It aims, fires, waits for the shot to resolve, writes `target/smoke-aim.png` and
`target/smoke-impact.png`, and closes after about 12 seconds of game time.

Tests cover timestep consistency, wind effects, fast-shot collision, damage falloff,
terrain repeatability and spawn pads across 100 seeds, crater limits, turn ordering,
round wind sharing, pause, misses, and a complete direct-hit/settling/victory sequence.

Manual playtest: play both turns, fire short and long shots, create a crater near a
cannon, inspect health and settling, try pause during flight, finish a match, and try
both restart modes. Desktop graphics/audio behavior still needs testing on each
target platform before distribution.

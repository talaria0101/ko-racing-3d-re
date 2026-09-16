# Deobfuscated race code (`java/`)

This directory holds readable Java reconstructed from the shipped
`KORa_17_612805.jar` (`Created-By: 1.7.0-b147`, MIDP-2.0 / CLDC-1.1,
JSR-184 / M3G). The original code is obfuscated to single and
double letter class names. Raw CFR output (when reproduced) goes to
`src/` (gitignored). What lives here is renamed, commented, and
checked against the data and the Rust port.

Scope for now: the race path only. That is the track model and its
detail layers, the mesh and node wrappers, and the race scene with
its cars, controller, and per-level config. Menus, career front end,
network, Bluetooth, ads, sound, and fonts stay obfuscated for later.

## How it was done

No CFR and no `javap` were available inside the sandbox (the system
`javap` wrapper hangs waiting on stdin and the JVM paths are outside
the walls). Everything below comes from a small pure Python class
file reader (`/tmp/dis.py`, `/tmp/clsinfo.py`, kept out of the repo)
that dumps the constant pool plus field and method tables and
disassembles `Code` attributes, combined with:

* `m3g_lwjgl` in https://github.com/shinovon/KEmulator : the M3G
  reference for what `VertexBuffer.setPositions` /
  `setTexCoords` do with scale and bias, and what
  `Transformable.getCompositeTransform` does (`T * R * S * custom`).
  In particular `VertexBuffer.getVertex` decodes a signed Java byte
  as `raw * scale + bias`, which is why the mesh files read every
  component as signed and add `128 * scale + bias`.
* the shipped assets themselves: every `.tl`, `.md`, `.hd`, `.ob`,
  `.map`, and model was parsed to the exact final byte, and the
  numeric ranges (tile edge 14, half tile 7, node scale 7.01) were
  confirmed against the geometry.

Where a method body has been fully walked at the bytecode level it
says so. Where it has not, the file keeps the complete field and
method inventory with the original obfuscated names in a mapping
comment, renames what is known, and marks the rest `TODO`. Nothing
is invented to fill a gap.

## Class map (race path)

| Original | This dir | Role | Status |
| --- | --- | --- | --- |
| `am` | `M3GNode.java` | M3G node wrapper: translation, rotation, scale, composite `Transform` | bodies verified |
| `at` | `MeshLoader.java` | mesh file to `VertexArray` / `VertexBuffer` / `Mesh` plus `Appearance` | header + scale/bias verified, appearance partly |
| `ar` + `a` | `TrackTile.java` | `.tl` parser, collision mesh, tile placement | parser + placement verified |
| `bc` | `MidDetail.java` | `.md` parser, seasonal texture swap, placement | parser + placement verified |
| `bp` | `HighDetail.java` | `.hd` parser, per-side mirroring, yaw | parser + placement verified |
| `ai` | `SceneryObject.java` | `.ob` parser, flag picks RGBA/RGB and the render path | parser + both paths verified |
| `bm` | `TrackCell.java` | one track cell: tile plus mid/high refs | structure verified, some helpers TODO |
| `bs` | `GameTrack.java` | `.map` parser and track model | parser verified, render/height partly |
| `cl` (`extends ca`) | `RaceCar.java` | one car on the track (player or AI) | inventory complete, physics TODO |
| `bt` (`extends y`) | `RaceController.java` | holds the track, the cars, and the camera transforms | inventory complete, loop TODO |
| `bd` (`extends bh`) | `RaceStage.java` | the race scene (HUD, states, frame switch) | inventory complete, states TODO |
| `bh` (`extends y`) | `RaceBase.java` | shared race screen base | inventory complete, bodies TODO |
| `r` (`extends y`) | `RaceConfig.java` | per-level race setup from `.001` | layout verified, UI TODO |
| `bq` | `Renderer.java` | thin `Graphics3D` wrapper (`render(Node, Transform)`) | bodies verified |
| `j` | `CameraRig.java` | camera-side transform holder passed into track rendering | fields verified, math TODO |

`y` (the screen base), `ca` (the car base), `bf` (tile cache), `ae`
(mid cache), `ap` (high cache), `b` (object list), `cf`/`de`/`bb`
(texture, model, and engine managers), `df`/`di`/`x`/`z`/`bj`/`bz`/
`ct` (vectors and helpers) are referenced but not yet renamed here.

## What this explains about the trackside screenshots

Timberton (`levels/ma1.map`, theme 4/autumn) on branch
`fix-missing-trackside-detail` shows three symptoms:

1. a giant foliage wall on the right (trees smeared into one tall
   block),
2. grey rocks and walls floating above the road,
3. thin guard rails crossing the track in mid air.

The placement code below pins one concrete mismatch behind symptom
1. `SceneryObject` has two render paths selected by the `.ob` flag
(`ai.b`, set on trees, bushes, and cacti). The normal path applies
the cell yaw (`bp` passes `0/90/180/270` and `ai` posts it about
`(0,0,1)`). The flagged path sets translation only and renders with
identity rotation. The Rust port (`scene.rs`) currently applies the
cell yaw to every high detail object, so every flagged tree is
rotated `arg * 90` degrees away from where the game puts it. Flat
billboard planes (`t1`: all `y = 0.15`, `0.36` wide, `0.53` tall
before the `7.01` node scale) turned edge on or piled face on read
as a wall. The fix is to render flagged objects with identity
rotation. Symptoms 2 and 3 are not yet pinned and are marked TODO
in `HighDetail.java` and `RaceCar.java`: the `.hd` second and third
triples (`scale`, `scale2`, always `1.0` in shipped data) are parsed
but ignored by both the game and the port for rendering, so they
are not the cause; the remaining suspects are the `z` offset
(`ar.b * (byte/100 - 1)`, with `t50.hd` placing `zc` `1.4` units
up) and mid detail models whose own geometry already floats
(`sh`: `z -1.05..-0.19`, i.e. `1.33..7.36` above ground after the
node scale).

## Files

Each file starts with its original name, superclass, and what was
checked. `TODO(obfuscated)` marks a method whose body has not been
walked yet. Field and method inventories are complete even where
bodies are TODO, so a later pass can fill them without relearning
the layout.

## Port gotchas (from review passes 1-5 over all 121 classes)

Proven against the bytecode; check here before "fixing" a divergence.

* Cells are centered on `(14x, 14y)` and span 7 units each way.
  `bm.<init>` multiplies the int cell by `ar.c` (14.0); tile and
  fence geometry is centered (`[-1,1]` times the 7.01 node scale).
* Every rotation is `+90 * arg` degrees about `+Z`, tiles, mid and
  high detail alike (`TrackTile`, `MidDetail`, `HighDetail`). The
  drivable-side mask rotates as `open[(d + arg) % 4]`. Matches
  KEmulator `postRotate` (counterclockwise positive).
* Composite order is translate, then rotate, then scale
  (`T * R * S`), scale 7.01 baked at load. Mesh bytes are signed:
  `raw * scale + bias`.
* Flagged high detail (trees/bushes/cacti with the `.ob` flag) keeps
  identity rotation and translation only; the `bp` yaw is dropped.
* `.hd` kind 0 is an intentional empty slot (`t38.hd`), not a bug.
* Booleans materialize as `iconst; goto` joins (see `jt` temps);
  port the resulting value, not the jumps.
* `fcmpg` vs `fcmpl` and `lcmp`/`dcmp` keep NaN semantics; the
  emitter prints which one, and the Rust port must match it
  (`total_cmp` is not `partial_cmp`).
* `idiv`/`irem` by zero throws on the device; 102 files use them
  (often `time / 60`). Rust panics too, but only in debug, so keep
  the same guards.
* No `jsr`/`ret` anywhere: no subroutines; `finally` is exception
  tables only. One synchronized class (`Bluetooth`): the game is
  effectively single-threaded.
* `/* pop: ... */` comments are obfuscator filler (built strings
  that are discarded). Do not port them as behavior.
* SMS unlock lives in `SysUtil`, Bluetooth multiplayer in
  `Bluetooth`, ads in `VservManager` (third-party SDK, 23 KB, not
  game logic, do not port).
* Saves use `RecordStore` (`Settings`, `MainMenu`); randomness is
  the global `KORa.rand`.
* Opponent ghosts (`GhostCar`) deserialize timestamp-gated frames
  with interpolation deltas; audio (`AudioPlayer` chain) debounces
  clip starts at 500 ms with per-name player caching.
* Same field name with different descriptors is one obfuscator
  slot; generated files disambiguate with suffixes (`a_String`,
  `b_Z`, `a_arrI`). `Gen*.java` is raw emitter output kept beside
  each curated file for review; `Obf*.java` is not hand edited.

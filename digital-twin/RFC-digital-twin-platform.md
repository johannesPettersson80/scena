# RFC: Digital Twin Platform — concept & architecture

- **Status:** Draft / concept — *for discussion, not a commitment.* No build implied.
- **Type:** Vision + early-stage RFC (a "prospect" — circulated to align on architecture before any work).
- **Date:** 2026-06-21
- **Depends on:** `scena` (renderer), `truST` (PLC realtime control).
- **Grounded:** claims checked against the scena + truST repos and online prior
  art (Codex review, 2026-06-21); real vs aspirational marked through the doc.

> This is a first-draft thinking document. Numbers, scope, and component
> boundaries are proposals to react to, not decisions. Nothing here changes
> `scena` until a separate, approved checklist says so.

---

## 1. Purpose

Sketch what a **digital-twin platform** built on `scena` + `truST` would look
like: a *living, physically-simulated* model of an industrial system that an
**AI can author and self-verify**, rendered by `scena`, controlled by `truST`.

"Living" is the load-bearing word: flow in a pipe happens because a pump pushes
fluid against gravity and friction — the visuals are a *consequence of
simulation*, not data fed in from outside.

## 2. The three pieces (system context)

| Piece | Role | Status |
|---|---|---|
| **scena** | Renderer — the *eyes*. Draws whatever state it's given. | exists |
| **truST** | PLC realtime — the *brain*. Runs control logic deterministically. | exists |
| **Digital twin** | The *body* + *nervous system*. Simulates the physical plant and wires everything together. | this RFC |

## 3. Core architecture — a closed control loop ("virtual commissioning")

The three pieces form a **loop**, not a pipeline. The PLC drives actuators, the
simulation computes the physical response, sensors feed back to the PLC, and
`scena` renders it. The defining property: **`truST` cannot tell whether it is
driving the real plant or the simulation** — same control logic, swapped body.
That is virtual commissioning / a control digital twin.

```
        ┌──────────────── truST (PLC realtime: control logic) ────────────────┐
        │ read inputs → run logic → write outputs                              │
        ▼                                                                      ▲
   actuator commands                                                     sensor readings
 (valve %, pump rpm, on/off)                                       (flow, pressure, level, temp)
        │                                                                      │
        ▼                                                                      │
   ┌────────────────────  I/O BUS / process image  ───────────────────────────┤
   │          maps PLC signals ↔ sim actuators/sensors, in realtime lockstep    │
        ▼                                                                      │
   Simulation engine (virtual plant) ── physics step ──► twin state ───────────┘
   rigid-body (gravity/friction/collision) + 1-D hydraulics (flow/pressure)     │
        │                                                                       ▼
        └──────────────── twin state ──────────────────────────────────► scena (render)
```

## 4. Components of the twin layer

The simulation engine is one of six parts; the others are what make it a *twin*
rather than a physics demo.

1. **Simulation engine** — the virtual plant. Computes plant response from
   actuator inputs + physics laws.
2. **I/O bus / process image** — *the heart.* The contract between `truST` and
   the sim: PLC outputs → sim actuators, sim sensors → PLC inputs. Realtime,
   lockstep. Also the seam that lets the real plant be swapped in for the sim
   unchanged.
3. **Realtime clock / scheduler** — steps PLC and sim *in sync* at the PLC scan
   rate. Supports realtime (1×), accelerated (what-if), and step/pause.
4. **Twin model** — the declarative description tying it together: entities +
   **physical properties**, the **I/O map** (which PLC signal ↔ which actuator/
   sensor), and the **state→visual bindings**. This is what the AI authors.
5. **Historian / telemetry** — records state over time → playback, time-scrub,
   what-if comparison, analysis.
6. **Real-plant data plane** *(optional; turns a simulator into a twin of a
   specific real machine)* — fuse actual sensor data alongside the sim to
   calibrate and detect anomalies ("real flow is 12% below physics → fouling").

Plus the **agent surface** spanning all of it (§6).

## 5. Boundaries & responsibilities (each piece stays reusable)

- **scena** → render only. *Renderer-only stance is preserved; no physics, no
  control logic enters scena.* This protects scena's reuse (and the trust
  platform that depends on it).
- **truST** → control only; agnostic to real-vs-sim.
- **simulation engine** → physics only; knows nothing about control or rendering.
- **twin model / orchestrator** → the *wiring* (I/O map, visual bindings, clock)
  and the AI surface. The only component aware of all three.

## 6. The AI-authoring surface (the differentiator)

The AI authors four things declaratively — **plant**, **mates** (assembly +
connections), **I/O map**, **visuals** — in intent terms (names, not
coordinates), consistent with scena's agent surface (stable JSON + CLI, no MCP):

```json
{ "schema": "scena.twin.v1",
  "fluids": [{"id":"water","density":998,"viscosity":1.0e-3}],
  "entities": [
    {"id":"tank-A","type":"tank","fluid":"water","level":0.8},
    {"id":"pump-1","type":"pump","curve":"3kW-centrifugal"},
    {"id":"pipe-1","type":"pipe","d":0.05,"length":4,"roughness":"steel"}],
  "mates": [
    {"a":"tank-A.outlet","b":"pipe-1.in"},     // geometry snaps AND water flows →
    {"a":"pipe-1.out","b":"pump-1.suction"} ],
  "io_map": [
    {"plc":"%Q0.0","drives":"pump-1.run"},
    {"plc":"%IW10","reads":"pipe-1.flow"}],
  "physics": {"gravity":[0,-9.81,0], "step":"realtime"},
  "visuals": [
    {"map":"pipe-1.flow","to":"flow_anim"},
    {"map":"pipe-1.pressure","to":"color"},
    {"map":"tank-A.level","to":"fill"}] }
```

### What makes this easy for an LLM — and what must be true

This is the **primary goal**, and it is *not* satisfied by the DSL alone. With
only `twin.v1`, an LLM would **invent** component types, pump curves, port names,
units, and PLC scaling — producing a plausible scene that renders but isn't a
valid closed-loop twin. Ease requires the LLM to get **bounded choices, never a
blank page**:

- **A constrained, discoverable catalog** — `catalog get` / `schema get` return
  the *exact* available component types, ports, and parameters. The LLM picks
  from these; it does **not** invent equations, curves, FMUs, or units.
- **Real verbs, not just a schema** — `twin validate` / `compile` / `simulate` /
  `verify` must exist and run, so the loop actually closes.
- **Repairable, structured errors** — every failure prescribes the fix
  (*"pipe-1.out is unmated — mate it to a `fluid_in` port"*) so the LLM converges
  instead of guessing.
- **Start tiny** — the MVP's fixed tank/pipe/pump catalog first; grow only once
  the loop is proven easy on a few types.

Until these exist, "an LLM can easily build a twin" is **aspirational** — they
are the first build priority.

### Assembly & topology via mating (typed ports)

scena already has CAD-style mating — `scene.mate(&a, "shaft", &b, "hub")` — and
imported glTF carries named **anchors** and **connectors** (`SceneAssetConnector`
in `src/assets/gltf/connectors.rs`; runtime frames in `src/scene/connectors.rs`).
The twin reuses this so the AI **never writes coordinates**: it mates named ports
and positions are solved.

The key move: a **mate carries both the geometry and the physics edge.** Mating
`pipe.out → pump.suction` snaps the parts *and* declares that fluid flows
pipe→pump. So `mates` replaces *both* per-entity `place:[x,y,z]` *and* a separate
`loop`/`from→to` topology — one declaration yields the visual assembly **and** the
simulation network.

**One extension — in the twin layer, NOT scena.** scena's connectors stay purely
geometric (snap frames; today's type-check is a shallow string-based
`allowed_mates`). The **twin catalog/compiler** owns the **typed** ports —
`mechanical` / `fluid_in` / `fluid_out` / `signal` — turning a mate into the right
physics edge (mechanical → rigid-body joint; fluid → flow connection) and
type-checking it (a fluid outlet cannot mate to a mechanical mount). scena gains
**no** pump/valve/fluid semantics — the twin compiles typed ports down to plain
scena connectors. The geometric mate + anchor machinery is reused unchanged.

### Worked examples (one schema, two physics domains)

**Factory cell — robot + conveyor (rigid-body):**

```json
{ "schema":"scena.twin.v1",
  "physics":{"engine":"rigid_body","gravity":[0,-9.81,0]},
  "entities":[
    {"id":"conveyor","type":"conveyor","size":{"length":4,"width":0.6}},
    {"id":"robot","type":"robot_arm","model":"import:ur10.glb"},
    {"id":"feed","type":"spawner","spawns":"box","rate":"1/3s"} ],
  "mates":[
    {"a":"conveyor.foot","b":"floor"},
    {"a":"robot.base","b":"floor.mount_2"},
    {"a":"feed.port","b":"conveyor.infeed"} ],
  "io_map":[ {"plc":"%Q0.0","drives":"conveyor.belt"}, {"plc":"%QW10","drives":"robot.joints"},
             {"plc":"%Q1.0","drives":"robot.grip"}, {"plc":"%I0.0","reads":"conveyor.eye"} ],
  "visuals":[ {"map":"conveyor.belt","to":"belt_scroll"}, {"map":"robot.joints","to":"pose"} ] }
```

**Air-conditioner — vapor-compression cycle (thermo-fluid):**

```json
{ "schema":"scena.twin.v1",
  "fluids":[{"id":"r410a","kind":"refrigerant","model":"R410A"}],
  "physics":{"engine":"thermo_fluid","model":"fmu:vapor_compression"},
  "entities":[
    {"id":"comp","type":"compressor","fluid":"r410a"},
    {"id":"cond","type":"heat_exchanger","hot":"r410a","cold":"outdoor"},
    {"id":"txv","type":"expansion_valve"},
    {"id":"evap","type":"heat_exchanger","hot":"indoor","cold":"r410a"} ],
  "mates":[
    {"a":"comp.discharge","b":"cond.in"}, {"a":"cond.out","b":"txv.in"},
    {"a":"txv.out","b":"evap.in"}, {"a":"evap.out","b":"comp.suction"} ],
  "io_map":[ {"plc":"%QW0","drives":"comp.speed"}, {"plc":"%QW2","drives":"txv.open"},
             {"plc":"%IW0","reads":"evap.suction_T"}, {"plc":"%IW2","reads":"evap.superheat"} ],
  "visuals":[ {"map":"*.refrigerant_T","to":"pipe_color"}, {"map":"*.flow","to":"flow_anim"},
              {"map":"indoor.temp","to":"kpi"} ] }
```

Same schema; the four fluid-port `mates` *are* the refrigerant loop;
`physics.engine` (or the component types) selects rigid-body vs thermo-fluid
under the hood. The AI writes no coordinates, no Rapier, no FMU XML.

**Self-verification on three axes** (the payoff — an AI can build a *controlled
physical world* and trust it):
- **Physical plausibility** — conservation of mass/energy; no impossible
  pressures or flow directions.
- **Control-loop correctness** — PLC commands pump on ⇒ flow actually starts;
  valve closes ⇒ pressure rises. (Tests the logic against the simulated plant.)
- **Render-reflection** — the visual matches the simulated state (the scene
  composition verifier already built for scena extends here).

### Coordinate & orientation handling

How the twin keeps imported assets right-way-up (a common real failure when CAD/
DCC tools export Z-up):

- **Units** are recorded on import — glTF anchors carry `source_units`
  (m/cm/mm/in/ft) (`src/assets/gltf/anchors.rs`), so scale mistakes (a 1700 mm
  pump imported as 1700 m) are catchable from metadata.
- **Up-axis:** glTF is Y-up *by spec*, so a correctly-exported asset is right by
  construction; scena assumes Y-up and does not guess an axis.
- **Wrong-up detection is geometric, caught by the verify loop** — not a metadata
  flag, because bad exports mislabel themselves:
  - catalog components carry a `base` / `mount` port;
  - on `verify`, mate `asset.base → floor` and check it **sits grounded**
    (min-Y ≈ 0, not floating or sunk) and that its **tallest extent is vertical**
    for things meant to stand. A sideways asset visibly tips and fails the check.
- **Self-repair:** on failure the LLM applies the standard correction (−90° about
  X for a Z-up source) and re-verifies — detecting *and* fixing a coordinate
  error on its own. Same loop, applied to orientation.

This needs no new subsystem: it reuses recorded `source_units`, the `base`
anchor/port, and the composition grounding check.

## 7. Scope tiers (what's realistic)

- **Tier 1 — tractable, real, real-time:** rigid-body dynamics (gravity,
  friction, collisions, joints) + **1-D hydraulic network** (flow, pressure,
  Darcy–Weisbach friction, pump curves, valves, tanks). This *is* "flow in a
  pipe under gravity and friction," and it runs live. **The proposed starting
  target.**
- **Tier 2 — harder:** thermal, particle/SPH fluids for visible free-surface
  flow, soft bodies.
- **Tier 3 — research/HPC:** full 3-D CFD. Offline / precomputed only; not in
  the realtime loop.

**Adopt existing solvers; do not write physics from scratch** — e.g. a mature
Rust rigid-body engine (Rapier) and an EPANET-style hydraulic network model.
The real engineering is the *coupling*, the *twin model*, and the *AI surface*.

## 8. Prior art & build-vs-adopt (researched 2026-06-21)

Verified against current literature/tooling, not recollection.

- **LLM-generated simulation is active prior art — but only in the *hard*
  form.** SimBench benchmarks LLMs generating complete multi-physics twins from
  natural language for the **Chrono** engine (generalizing to ANSYS / ABAQUS /
  OpenFOAM / IsaacSim / pyBullet); the LLM emits **simulator-specific
  code/models**, graded by an **LLM-as-judge + rules + human-in-the-loop**.
  **ChronoLLM** customizes LLMs to emit PyChrono code, but generated scripts are
  *"rarely perfect."* Takeaway: the demand is real and the gap is ours — we've
  found **none** that ship an **easy declarative DSL + deterministic
  self-verification + integrated rendering** (generic DT platforms exist but are
  IoT/cloud/ML-shaped, not this LLM-first renderer+PLC loop). We *mine SimBench's
  failure taxonomy for our verifier* rather than reinvent it.
- **Physics: adopt.** Rapier (Rust/WASM rigid body); Project Chrono / PyChrono
  (mature open multiphysics) reachable as FMUs for richer domains.
- **Coupling is solved, in Rust.** A pure-Rust FMI stack exists — `fmi`
  (FMI 2.0/3.0), `fmi-sim` (FMU co-simulator), `fmi-export` (build FMUs in
  Rust). FMI is the industry standard for virtual commissioning (PLC ↔ plant).
  It becomes the **truST ↔ sim I/O bus** and lets any FMI solver plug in.
- **Full twin platforms: don't adopt.** OpenTwins, interTwin / itwinai, Eclipse
  Ditto, Azure Digital Twins are Python / IoT / composition / cloud-shaped —
  wrong fit for easy + LLM-first + renderer-only + realtime.

**Decision:** adopt the physics engines + the FMI bus; **build** the easy
`twin.v1` DSL, the deterministic author→validate→simulate→verify→repair loop,
and the scena integration — the trio that is genuinely still open.

Sources: [SimBench (arXiv 2408.11987)](https://arxiv.org/abs/2408.11987) ·
[ChronoLLM (Springer s11044-026-10152-x)](https://link.springer.com/article/10.1007/s11044-026-10152-x) ·
[`fmi` crate](https://crates.io/crates/fmi) · [`fmi-sim`](https://docs.rs/fmi-sim/latest/fmi_sim/) ·
[OpenTwins](https://www.sciencedirect.com/science/article/pii/S0166361523001574) ·
[FMI co-sim for DTs (Nature s41598-025-28466-9)](https://www.nature.com/articles/s41598-025-28466-9)

## 9. Complete tech stack

Tagged **[exists] / [adopt] / [build]**. **The LLM only ever authors
`twin.v1`** — everything below is *compiled* from it (no PyChrono code, no FMU
XML by hand — the heavy formats SimBench shows LLMs fail at).

| Layer | Role | Tech | Status |
|---|---|---|---|
| **Agent surface** | the only thing the LLM touches | `twin.v1` JSON + CLI + stable JSON out; templates; self-verify loop | **build** |
| **Twin orchestrator** (`scena-twin`) | compiles intent → all below; runs the loop | Rust: schema+validation, compiler, I/O map, realtime clock, state→visual binding, verifier, historian | **build** |
| **Control plane** | control logic (brain) | **truST** (PLC realtime), via FMI | **exists** |
| **I/O bus** | couples control ↔ physics, lockstep | **FMI/FMU** via pure-Rust `fmi` / `fmi-sim` / `fmi-export` | **adopt** |
| **Simulation plane** | physics (body) | **Rapier** (rigid body); 1-D hydraulics (Rust build *or* FMU); **Chrono / OpenModelica** as FMUs | **adopt** + thin **build** |
| **Render plane** | visuals (eyes) | **scena** (wgpu + CPU fallback, WASM) via `scene_recipe.v1` | **exists** |
| **Data plane** *(optional)* | real-plant fusion + history | MQTT / OPC-UA / REST adapters; historian (embedded TSDB or external) | **build** |
| **Foundation** | runtime | **Rust** throughout; wgpu (Vulkan/Metal/DX/GL/WebGL2/WebGPU); native + WASM + headless | **exists/adopt** |

The closed loop, concretely:

```
LLM ─► twin.v1 ─► scena-twin compiles to:
                  ├─ sim model       (Rapier bodies + 1-D hydraulics / FMUs)
                  ├─ I/O map         (PLC signal ↔ actuator/sensor)
                  └─ scene_recipe.v1 (for scena)

  each realtime tick:
    truST out ─FMI─► sim actuators ─► sim step ─► sim sensors ─FMI─► truST in
                                       │
                                  twin state ─► scena (render) + historian (record)

  self-verify: run sim ► check {physical · control-loop · render-reflection} ► report ► LLM repairs twin.v1
```

**FMI is the spine** — truST, the sim, and any third-party solver share one bus.

**Deployment (one stack, three modes):** native realtime (full loop) · WASM/
browser (sim + scena, accelerated what-if) · headless (CI, snapshots, agent
self-verify).

**Open stack choices:** 1-D hydraulics build-vs-FMU · Rapier alone vs +Chrono ·
FMI vs a lighter native Rust trait for the realtime path · historian embedded
vs external.

## 10. Cross-cutting concerns

Beyond the architecture, these must be designed in — their *absence* is what
sinks a twin platform. Tagged *(reuse)* / *(new)* / *(later)*.

**Trustworthy simulation**
- **Units & dimensional consistency** *(reuse: verify loop)* — catch mixed
  m/mm or °C/K across the *whole* model, not only on import.
- **Numerical stability & determinism** *(new)* — NaN/Inf/divergence guards so
  the verifier fails a *blown-up* sim, not just an "implausible" one; plus
  reproducibility (same inputs → same outputs; seeds; deterministic ordering) so
  the self-verify loop is itself trustworthy.

**Realistic fidelity** (so truST is tested against signals it will actually see)
- **Sensor & actuator realism** *(new)* — noise, latency, sample rate,
  quantization, drift; actuators that ramp/lag, not instant.
- **Fault injection** *(new)* — stuck valve, leak, blockage, sensor failure,
  degradation. Core value-add: test failure scenarios and the PLC's
  fault-handling.

**Trust & safety**
- **Permissions / control modes** *(new)* — read-only viewing vs. driving
  actuators; the safety-critical boundary if the loop ever drives a real plant
  (the "trust" in truST).
- **Catalog provenance & introspection** *(new)* — the AI discovers component
  types (`catalog get`, like `schema get`); each type's physics is sourced and
  validated with provenance (as scena disciplines asset provenance).

**Later tiers** (flagged, not yet designed)
- **Real↔sim calibration / anomaly detection** *(extend: data plane)* — fit sim
  parameters to real data; flag divergence. Turns a simulator into a twin of a
  *specific* machine.
- **Scale: sim LOD + multi-rate co-sim** *(new)* — detail the cell-of-interest,
  coarse elsewhere; subsystems at different rates so a whole plant stays realtime.
- **State persistence & schema versioning** *(plumbing)* — save/load; twin.v1→v2
  migration.

## 11. Real-time, multi-rate & smooth motion

The defining real-time problem: discrete control (a PLC scan) must become
*smooth continuous motion* in the HMI without jerking once per scan. **Never pipe
PLC outputs straight to the render** — that snap *is* the jerk. The fix is three
decoupled clocks, and smoothness comes from **simulating the dynamics, not a
rendering trick**.

**Three clocks:**
1. **Control — ~50 ms (truST).** Writes setpoints/targets each scan. Discrete;
   never rendered directly.
2. **Simulation — fine (≈1–5 ms).** Runs the actuator/drive **dynamics** — a
   robot joint follows a **motion profile** (trapezoidal / S-curve; velocity /
   acceleration / jerk limited) toward each target, exactly as a real servo drive
   does. Smooth *and* physically faithful (shows real accel/decel).
3. **Render — 60–144 FPS (scena).** Samples the sim; interpolates the last mile
   (`lerp` positions, `slerp` orientations) so render-vs-sim mismatch is invisible.

**Worked example — robot, 50 ms scan:** the PLC commands "go to θ" every 50 ms;
the simulated drive *moves* the joints to θ along a motion profile at the sim
rate; scena samples that. The robot glides A→B and the 50 ms boundaries vanish —
because the *simulation* moved it, not a setpoint snap. *(Fallback with only
logged setpoints and no dynamics: spline the setpoints and sample — smooth but
not dynamically faithful. Prefer simulating the dynamics — it's why the twin has
a physics layer.)*

**The same shape recurs everywhere and must all be handled:**
- **Actuators:** a valve commanded `0→1` animates over its real ~800 ms stroke;
  a motor ramps — simulate the actuator, don't render the command.
- **On/off & state:** lights, clamps, status animate over their real transition.
- **Sensor jitter:** real telemetry doesn't arrive exactly on period — timestamp
  it and **resample to a smooth timeline**, don't render on arrival.
- **Latency vs overshoot:** interpolation lags (renders the past); extrapolation
  is lower-latency but overshoots on direction changes — a **per-signal** choice.
- **One timeline:** live, replay, and scrub share a single clock model so motion
  stays smooth across pause/seek, and the sim, render, and historian agree on "now."

### More cases (the same tell)

- **Sim can't keep realtime → don't lie about speed.** On a slow backend
  (WASM/lavapipe) the sim falls behind; render-every-step is slow-motion,
  skip-steps stutters. Decouple sim-time from wall-clock, fixed-step, and show an
  honest "0.7× realtime" (or drop to a coarser model) — never silently slow or
  stutter the world.
- **Runtime attach (pick-and-place).** A gripped box must ride the gripper with
  zero lag — a real runtime kinematic mate (parented to the gripper frame),
  released cleanly on drop; not a per-frame re-resolve.
- **Camera-follow judder.** Following the end-effector with a naive lerp shakes
  the whole HMI — use a critically-damped follow with a deadband, decoupled from
  the sim rate.
- **Time-scrub between sparse frames.** Interpolate between historian snapshots,
  per-signal: `lerp` continuous values, zero-order-hold discrete states (a valve
  is open or shut, never 40% mid-scrub).
- **Startup transient.** t=0 is at rest; the first seconds are pressures
  equalizing / the robot homing — show a warm-up/settle window or boot from a
  saved steady state; don't present the transient as the operating state.
- **Quantized sensor → steppy gauge.** Filter for *display*, keep raw for
  *logic* — the display signal is not the control signal.
- **Fluid fill/transport delay.** Pump on ≠ flow everywhere now — model
  fill/transport delay so flow propagates; don't snap "flow on" across the network.
- **Replay→live / reconnect teleport.** Blend/re-sync on the transition so state
  doesn't jump after a dropout or a mode switch.
- **Long-run drift.** Multi-day runs leak energy / drift; apply periodic
  constraint correction / re-anchoring.

**Principle: snapping a setpoint to the screen is the tell of a fake twin;
simulating the drive/actuator is what makes it both real and smooth.**

## 12. Asset pipeline & catalog components

A catalog component is **not** a 3D model. It is:

```
geometry  +  typed ports (on anchors)  +  physics spec (model + params, sourced)  +  default visuals
```

The AI composes from these — it never models 3D or writes equations.

### Geometry — three tiers

- **Tier 1 — procedural / parametric (the workhorse, ~80% of a plant).** Pipes,
  tanks, ducts, frames, conveyors, valves, flanges, structural steel — generated
  from parameters using scena's primitives. The *drawn* dimensions **are** the
  *physics* dimensions (no visual/sim mismatch). Primary source.
- **Tier 2 — imported CAD / glTF (specific branded equipment).** Manufacturer CAD
  (STEP→glTF) or model libraries, via scena's import (anchors, connectors,
  `source_units`, asset-doctor, provenance). The coordinate/units handling (§6)
  keeps these upright and to-scale.
- **Tier 3 — AI-generated 3D (props / filler only).** Text/image→3D for scenery.
  *Never* for a part whose dimensions drive physics — imprecise, weak provenance;
  flagged visual-only.

### Physics — a **model** + **parameters**

The **model** (equations) is written once and reused (`control_valve` knows
`flow = f(opening, ΔP, Cv, characteristic)` and stem lag). The **parameters** are
this component's data. So "is it linear?" and "what's its response time?" are
named params, not new code:

```json
{ "type":"valve", "model":"control_valve",
  "flow":{"cv":12.0,"characteristic":"equal_percentage","rangeability":50},
  "dynamics":{"stroke_time_s":0.8,"response":"first_order_lag","deadband_pct":0.5},
  "leakage_pct":0.01, "fail":"closed", "units":"SI",
  "provenance":{"source":"Fisher EZ datasheet","url":"...","sha256":"..."} }
```

- `characteristic` = `linear` / `equal_percentage` / `quick_opening`, or a custom
  curve as a point array. `stroke_time_s` feeds the lag that animates the valve
  over its *real* stroke instead of snapping (ties to §11).
- **Save:** the declarative spec above (curves as point arrays). For portability,
  a component's physics can be **exported as an FMU** (`fmi-export`).
- **Import (four routes):** (1) fill params for a known model; (2) **from a
  datasheet** — an LLM extracts Cv / characteristic / stroke-time (strong AI
  angle); (3) **a whole model as an FMU** (vendor / OpenModelica) — the literal
  "import the physics," reusing the *same FMI* as the truST↔sim bus; (4) **fit to
  measured data** (the calibration path).
- **Validation & provenance** — an asset-doctor for physics: range-check (Cv > 0,
  consistent units, recognized characteristic) and require provenance; unsourced
  or out-of-range physics is flagged, so the AI can't silently ship a made-up part.

### Scale

Few *types*, many *instances*: Tier-1 parametric + **GPU instancing**
(`InstanceSet`) + LOD means thousands of pipes/fittings are repeated instances,
not modeled individually.

### Build vs reuse

**Build:** the parametric generators, the port/physics **wrapping + registration**
tooling, the physics validator. **Reuse:** scena's primitives, glTF import,
anchors / connectors / units, asset-doctor, instancing, provenance, and the FMI
stack — most of the pipeline already exists.

**Unification:** FMI is both the co-sim bus *and* the portable physics-package
format — a component's behavior is a built-in model (params) or an imported FMU,
same standard.

## 13. I/O bus contract — truST ↔ sim (deep dive)

The architectural heart: how the PLC and the simulation exchange signals in
lockstep without instability.

**1. Signal model (the process image).** The bus is a typed signal table; each
signal has an address (`%Q0.0`, `%IW10`), a direction, a type (`bool`/`int`/
`real`), an engineering unit, and a raw↔EU **scaling**. Two directions only:
- **PLC output → sim actuator** (`pump.run`, `valve.opening`, `robot.joints`)
- **sim sensor → PLC input** (`flow`, `pressure`, `level`, `photo_eye`, `encoders`)

The `io_map` in `twin.v1` declares the bindings.

**2. Scaling lives in the twin layer (truST has none).** Verified: truST's
process image is **byte buffers + typed reads/writes — no engineering-unit
scaling**. So the raw↔EU mapping (e.g. `0–27648` ↔ `0–10 bar`) is the **twin
bus-adapter's** job — pinned per-signal and validated — *not* something to assume
truST does. A classic bug source, so it is explicit and checked.

**3. Timing / exchange (the stability problem).**
- **Synchronous, sub-stepped:** each PLC scan (≈50 ms) the bus exchanges I/O,
  then the sim runs many fine steps (≈1 ms) across the scan. PLC outputs are held
  **zero-order** across the scan; sensors are sampled at scan end.
- **Direct-feedthrough / algebraic loops:** if a PLC output instantly affects the
  sensor it reads in the same step → oscillation. Mark signals with/without
  direct feedthrough (FMI's distinction); reduce the comm step or iterate where needed.
- **Multi-rate:** subsystems may step at different rates; the scheduler
  coordinates and resamples at the bus boundary.

**4. Determinism modes.** *Realtime* = best-effort, may drop steps (reports
honest speed, §11). *Verify* = fixed-step, fixed sample points and ordering →
reproducible. The verifier runs in Verify mode.

**5. Startup handshake.** PLC and sim agree on `t=0` and initial I/O; a settle
window precedes "operating" (avoids the startup-transient garbage, §11).

**6. Safety & edges.** Out-of-range actuator commands are **clamped + flagged**;
unmapped signals are a validation error; a **watchdog** faults the loop if the sim
can't answer within the scan; divergence trips the numerical-stability guard (§10).

**7. Transport — native IoDriver first, FMI later.** Verified: truST today has
**no FMI** and couples via a native **`IoDriver`** that already supports simulated
I/O (`%Q → %I`). So the MVP bus is the **native truST IoDriver bridge** — the
proven path. **FMI** (`fmi` / `fmi-sim`; the latter is work-in-progress) is
adopted **later** for external / portable solvers — *not* the hot-loop spine
before the native bridge is proven.

**8. Still to specify (co-sim correctness).** Concrete rules for algebraic-loop
resolution, solver-budget overrun, signal timestamping, rollback/iteration on
rejected steps, sample ordering, and deterministic-replay evidence. Named here;
not yet designed.

## 14. Analyze surface — how the LLM reads a twin

The LLM does **not** look at the render (it can't reliably judge one — proven
the hard way). Its "eyes" are structured, queryable surfaces:

- **State report** — per-entity current values (all channels), machine-readable.
- **Verification report** — the three axes (physical / control-loop / render),
  each with pass/fail, residuals, and a **confidence** value. The primary
  "is it right?" signal.
- **Explainability / causal queries** — *"why is pump-2 in fault?"* → a causal
  trace (`temp>90 → alert → status=fault → light red`), not just state.
- **Experiments API** — *analyze = run what-ifs*: `experiment {inject:
  pump-2.fault, for: 60s}` → a **structured delta** (`indoor.temp +4 °C,
  COP −30%, alert X`). Sweeps and fault campaigns return tables the LLM reasons over.
- **History queries** — any signal over time from the historian, structured.
- **Twin→NL summary** — describe the twin/state back in words for confirmation.
- **Confidence always attached** — calibrated vs uncalibrated, inside vs outside
  model validity; never implies certainty it lacks (§10).

Principle: everything the LLM needs to reason about a twin is structured and
queryable; the render is for humans.

## 15. UI/UX — the human app

The 3D twin is the star; light chrome around it; browser/WASM.

```
┌ status summary ───────────────────────────────────────────────┐
│ entities/views/layers │   3D TWIN (scena, live)   │ inspector  │
│                       │   click-to-select          │ (state,    │
│                       │   flow / status / heatmaps │  history,  │
│                       │                            │  controls) │
├───────────────────────┴────────────────────────────┴───────────┤
│  ◀──●────▶  timeline / scrub   [▶ live] [speed ▾]               │
│  💬  ask the AI: "what if pump 2 fails?"                         │
└─────────────────────────────────────────────────────────────────┘
```

- **Five interactions:** select (click in 3D), scrub (timeline), what-if (run a
  scenario), ask-AI (NL authors/edits the twin), views/layers.
- **Easy principles:** direct manipulation (the 3D *is* the navigation),
  progressive disclosure, status legible at a glance, polished-by-default (the
  rendering-quality work), no 3D skill required.
- **AI woven in:** the NL bar drives the same authoring the LLM uses headlessly;
  the AI's **confidence** (from §14) is shown, not hidden.
- **Form factor:** WASM/browser; an embeddable web component
  (`<twin-viewer src="twin.json">`, like `<scena-viewer>`); role-based (viewer /
  operator / engineer).

## 16. MVP — first vertical slice

Prove the whole loop on the *smallest* controlled physical system before any
breadth. **Proposed slice: a pumped tank with level control.**

- **System:** `tank ← pipe ← pump`, one controller. truST: *if level < setpoint,
  pump on*. Physics: gravity + pipe friction + pump curve set the fill rate; the
  level rises and holds at setpoint. One actuator (`pump.run`), one sensor
  (`tank.level`).
- **Exercises every plane:** 1-D hydraulics (sim), the I/O bus (`pump.run` ↔
  `level`, scaled, lockstep), the closed control loop (truST), smooth motion
  (level rises continuously, pump animates — no jerk), the verifier (mass
  conservation + control-loop: pump-on → level-rises → setpoint held), and scena
  (render).
- **Stack — smallest proof, no FMI/Chrono:** a **deterministic in-process 1-D
  tank/pipe/pump** model + the **native truST IoDriver** bridge (truST already
  depends on `rapier3d`, so rigid-body is in-house). FMI/Chrono/OpenModelica come
  later, once one slice is real.
- **Flow:** LLM authors `twin.v1` from intent → compile → simulate with truST via
  IoDriver (or a stub controller) → verify (3-axis, headless) → render one view →
  repair loop.
- **Success:** an LLM builds the tank-pump-level twin from the **public surface**,
  it simulates correctly (level holds, mass conserved), renders **smoothly**, and
  the verifier passes — on **one backend**.
- **Explicitly out:** catalog breadth, multiple domains, calibration, scale, the
  full UI, AI-generated assets — those follow once one slice is real.

This is the depth that turns the vision into a buildable, de-risked plan.

## 17. Constraints & risks

- **Realtime lockstep is the hard part.** The sim step must complete inside
  `truST`'s scan-cycle budget. This bounds sim complexity — and is the concrete
  reason for 1-D hydraulics + rigid-body rather than live 3-D CFD.
- **Deployment tiers:** native realtime (full loop) vs. a slower **WASM
  what-if** mode in the browser (sim + scena, accelerated/non-hard-realtime).
  WASM matters for the trust-platform direction.
- **Magnitude:** this is a multiphysics *platform*, not a feature — a
  multi-quarter effort even using off-the-shelf solvers.

## 18. Non-goals (explicit)

- Not changing scena's renderer-only scope. Physics lives in a sibling layer.
- Not writing a physics engine from scratch.
- Not live 3-D CFD.
- Not absorbing control logic into the twin — `truST` owns control.

## 19. Open questions (to resolve before any RFC→design promotion)

1. Confirm **Tier 1 (rigid-body + 1-D hydraulics, realtime)** as the target,
   3-D CFD explicitly out.
2. The **I/O bus contract** between `truST` and the sim — protocol, signal
   typing, scan-rate coupling. (Probably the first deep-dive.)
3. Which solvers to **adopt vs. port vs. build**, with the dependency-boundary
   rule scena already uses.
4. **WASM/browser** as a first-class target, or native-realtime first?
5. Is the **`twin.v1` schema** a new layer that *compiles to* `scene_recipe.v1`
   (proposed), or an extension of it?
6. Where exactly the **real-plant data fusion** line sits (simulator vs. true
   twin of a specific machine).

## 20. Suggested next step

With the architecture, the I/O bus contract (§13), and the MVP slice (§16) now
specified, the concrete next step is to **build the MVP vertical slice** — the
pumped-tank level-control loop, end to end (author → simulate → verify → render)
on one backend. That single slice proves or breaks the whole design before any
breadth is built. Alternatively, pressure-test the architecture with a
multi-agent design fan-out first, given its size.

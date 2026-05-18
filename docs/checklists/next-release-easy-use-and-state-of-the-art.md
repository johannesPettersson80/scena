# scena post-1.3.0 — Easy-use + state-of-the-art roadmap

Created: 2026-05-18

Status: planning. v1.3.0 is the easy-scene-setup + auto-framing release;
this document lists the work for the release(s) after that.

scena's signature is **easy to use**. The prioritized list below is the
gap inventory between "Rust renderer that works" and "easier than
Three.js, more accurate than model-viewer." Items are grouped by the
shipping rounds in §3 — Rounds A–D are the ease-of-use signature work;
the renderer state-of-the-art and differentiator sections after that are
the separate quality-bar arc.

This is a planning document, not a contract. Items become contracts as
they're picked up; each round will get its own narrow implementation
checklist (the way `easy-scene-setup-and-auto-framing.md` was structured
for v1.3.0).

---

## 1. Two corrections to existing direction

Before adding anything new, fix two defaults the current code chose
wrong:

- [ ] **Default tonemapper is Khronos PBR Neutral, not AgX.** AgX is
      correct for filmic content; PBR Neutral was released by Khronos
      in 2024 specifically for product/e-commerce/digital-twin use
      cases. Blender adopted it, Filament integrated it, model-viewer
      exposes it as `tone-mapping="neutral"`. It preserves
      brand-accurate sRGB colors — exactly what a digital-twin viewer
      needs. Keep AgX available; default is PBR Neutral.
- [ ] **Auto-framing is the default, not opt-in.** model-viewer
      auto-frames on every model load; drei's `<Bounds fit clip observe>`
      is one of its most-used components. Today scena treats
      `frame_bounds()` as an explicit call. Make it the default when no
      camera is specified; explicit `Camera::default()` should mean
      "compute a good view"; manual placement is the opt-out.

---

## 2. Tier 1 — "write a name, not a number" gaps

The signature pattern: wherever the library currently forces users to
write raw coordinates or magic floats, give them a named primitive.

### 2.1 `Color` has no named constants

Every example writes `Color::from_srgb_u8(80, 180, 220)` or
`Color::from_linear_rgb(0.014, 0.017, 0.024)`. Eleven occurrences in
`examples/`. Users have to guess RGB triples for "warm yellow" or
"studio backdrop."

```rust
// today
Color::from_linear_rgb(0.014, 0.017, 0.024)

// easy
Color::STUDIO_BACKDROP            // or Color::CHARCOAL
Color::from_hex("#1a1d28")        // for designers
Color::from_kelvin(3200.0)        // for light color temperature
```

- [ ] Constants: `WHITE`, `BLACK`, `GRAY`, `LIGHT_GRAY`, `DARK_GRAY`,
      `CHARCOAL`, `STUDIO_BACKDROP`, `WARM_WHITE`, `COOL_WHITE`,
      `RED`, `GREEN`, `BLUE`, `ORANGE`, `YELLOW`, `CYAN`, `MAGENTA`.
- [ ] `Color::from_hex(&str)` — `"#1a1d28"` and `"1a1d28"` both work;
      typed error on malformed input.
- [ ] `Color::from_kelvin(temp_k)` — for light color temperature in
      Kelvin (2700–6500K is the useful range).

### 2.2 `PerspectiveCamera` lens presets

Every example writes
`PerspectiveCamera::default().with_aspect(width as f32 / height as f32)`.
There's no way to say "wide angle" or "portrait lens" without finding
the radians.

```rust
// today
PerspectiveCamera::default()                              // what FOV? opaque
PerspectiveCamera::default().with_fov(1.04719)            // radians, no thanks

// easy
PerspectiveCamera::wide_angle()    // ~24mm equivalent, ~84° FOV
PerspectiveCamera::standard()      // ~50mm equivalent, ~46° FOV (default)
PerspectiveCamera::portrait()      // ~85mm equivalent, ~28° FOV
PerspectiveCamera::telephoto()     // ~135mm equivalent, ~18° FOV
PerspectiveCamera::standard().with_fov_degrees(60.0)  // escape hatch in degrees
```

- [ ] `wide_angle`, `standard`, `portrait`, `telephoto` lens presets.
- [ ] `with_fov_degrees(deg)` escape hatch.

### 2.3 Drop the `with_aspect` boilerplate

Every example writes `.with_aspect(width as f32 / height as f32)`
because `frame_bounds` overwrites it anyway. Make this implicit:
`FramingOptions::viewport(w, h)` sets aspect automatically; document
the side effect; drop the boilerplate from every example.

```rust
// today (every example)
let camera = scene.add_perspective_camera(
    scene.root(),
    PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
    Transform::default(),
)?;

// easy
let camera = scene.add_perspective_camera_default()?;
// or
let camera = scene.add_perspective_camera(
    scene.root(),
    PerspectiveCamera::standard(),
    Transform::default(),
)?;
```

- [ ] `Scene::add_perspective_camera_default()` convenience.
- [ ] Document that `FramingOptions::viewport(w, h)` writes the
      camera aspect; users can omit `with_aspect`.

### 2.4 `Transform` rotations in degrees

`Transform::at(Vec3::new(...))` works for translation. Rotation
requires `Quat::from_axis_angle(Vec3::Y, deg.to_radians())` — not a
thing a beginner reaches for.

```rust
// today
Transform {
    translation: Vec3::new(1.0, 0.0, 0.0),
    rotation: Quat::from_axis_angle(Vec3::Y, 0.7853),
    ..Default::default()
}

// easy
Transform::at(Vec3::new(1.0, 0.0, 0.0)).rotated_y_degrees(45.0)
Transform::default().rotated_x_degrees(-90.0)   // glTF Y-up → CAD Z-up
Transform::looking_at(target_position, Vec3::Y) // node faces a point
```

- [ ] `Transform::rotated_x_degrees(deg)` / `rotated_y_degrees` /
      `rotated_z_degrees`.
- [ ] `Transform::looking_at(target, up)`.

### 2.5 Named camera views on `FramingOptions`

(This is the work prompted by the v1.3.0 connector-angle drift.)

```rust
// today
FramingOptions::new().look_from(Vec3::new(-0.4398, 0.3051, 0.8447))

// easy
FramingOptions::new().three_quarter_front_left()
FramingOptions::new().azimuth_elevation(-27.5, 17.8)  // 28° left, 18° up
```

- [ ] `front`, `back`, `left`, `right`, `top`, `bottom`,
      `three_quarter_front_left/right`, `three_quarter_back_left/right`.
- [ ] `azimuth_elevation(az_deg, el_deg)` escape hatch.

### 2.6 Light presets

`DirectionalLight`, `PointLight`, `SpotLight` are public types but
each example would have to set intensity, color, and direction by
hand. `add_studio_lighting` covers the 3-point case; individual lights
have no shorthand.

```rust
// today
DirectionalLight {
    color: Color::from_srgb_u8(255, 244, 220),
    intensity: 5.0,
    direction: Vec3::new(-0.3, -1.0, -0.2).normalize(),
}

// easy
DirectionalLight::sun()              // warm white, bright, default sun angle
DirectionalLight::key_light()        // 3-point key
DirectionalLight::fill_light()       // 3-point fill
DirectionalLight::rim_light()        // 3-point rim/back
PointLight::softbox()                // big soft fill
PointLight::bulb_warm()              // 2700K incandescent
PointLight::bulb_cool()              // 5600K daylight
```

- [ ] `DirectionalLight::sun`, `key_light`, `fill_light`, `rim_light`.
- [ ] `PointLight::softbox`, `bulb_warm`, `bulb_cool`.
- [ ] (Optional) `SpotLight::stage_spot`, `accent`.

### 2.7 `MaterialDesc` PBR presets

Examples use `MaterialDesc::unlit(Color)` and
`MaterialDesc::line(Color, width)`. For PBR there's no shortcut —
users would build the full descriptor with `metallic`, `roughness`,
`clearcoat`.

```rust
// today
MaterialDesc {
    base_color: Color::from_srgb_u8(140, 145, 150),
    metallic: 0.9,
    roughness: 0.35,
    ..Default::default()
}

// easy
MaterialDesc::matte(Color::WHITE)
MaterialDesc::plastic(Color::RED)
MaterialDesc::metal(Color::from_hex("#c0c0c0"))
MaterialDesc::brushed_steel()
MaterialDesc::chrome()
MaterialDesc::clear_glass()
MaterialDesc::frosted_glass()
MaterialDesc::leather(Color::DARK_GRAY)
MaterialDesc::rubber()
```

- [ ] `matte(Color)`, `plastic(Color)`, `metal(Color)`, `leather(Color)`,
      `rubber()`.
- [ ] `brushed_steel`, `chrome`, `clear_glass`, `frosted_glass`.

### 2.8 `Background` named scheme

`renderer.set_background_color(Color::from_linear_rgb(0.014, 0.017, 0.024))`
appears in the showcase example. There's no named scheme.

```rust
// today
renderer.set_background_color(Color::from_linear_rgb(0.014, 0.017, 0.024));

// easy
renderer.set_background(Background::DarkStudio);
// variants: Studio, DarkStudio, NeutralGray, White, Black, Sky, Transparent, Custom(Color)
```

- [ ] `Background` enum + `Renderer::set_background(Background)`.

### 2.9 `OrbitControls` named damping presets

`OrbitControls::new(Vec3::ZERO, 2.0).with_damping(0.15)` — what does
0.15 mean? Five examples copy-paste the same value.

```rust
// today
OrbitControls::from_framing(framing).with_damping(0.12)

// easy
OrbitControls::from_framing(framing).cinematic()      // heavy damping, smooth
OrbitControls::from_framing(framing).snappy()         // light damping, responsive
OrbitControls::from_framing(framing).presentation()   // medium + slow auto-rotate
OrbitControls::from_framing(framing).turntable(6.0)   // auto-rotate, 6 RPM
```

- [ ] `cinematic`, `snappy`, `presentation`, `turntable(rpm)`.

### 2.10 `AutoExposureConfig` scenario presets

`with_ev_range(min, max)` + `with_highlight_guard(percentile, target_lum)`
requires the user to know what those numbers mean. No scenario presets.

```rust
// today
AutoExposureConfig::default()
    .with_ev_range(-2.0, 4.0)
    .with_highlight_guard(0.98, 0.85)

// easy
AutoExposureConfig::product_studio()    // tight EV range, clean highlights
AutoExposureConfig::indoor()            // wider EV range, warm bias OK
AutoExposureConfig::outdoor()           // wide range, anti-blowout guard
AutoExposureConfig::mixed()             // default, conservative
```

- [ ] `product_studio`, `indoor`, `outdoor`, `mixed` scenarios.

---

## 3. Tier 2 — related ease-of-use gaps

Same spirit (named primitive over numeric API), different shape.

### 3.1 Bundled Khronos sample loader

Every example points at `tests/assets/gltf/...`. Three.js shows
`gltfLoader.load('Duck.glb')` against a CDN; model-viewer ships sample
URLs. Bundle the canonical Khronos samples behind a feature flag.

```rust
// today
assets.load_scene("tests/assets/gltf/khronos/WaterBottle/glTF-Binary/WaterBottle.glb").await?

// easy
Assets::khronos::water_bottle().await?
Assets::khronos::damaged_helmet().await?
Assets::khronos::boom_box().await?
Assets::khronos::dragon_attenuation().await?   // transmissive control
```

- [ ] `scena = { features = ["khronos-samples"] }` feature flag.
- [ ] `Assets::khronos::*` namespaced loaders for at least: WaterBottle,
      DamagedHelmet, BoomBox, DragonAttenuation, ToyCar, FlightHelmet.

### 3.2 Bounds-relative `OrbitControls` zoom

`OrbitControls::new(Vec3::ZERO, 2.0)` — `2.0` is "2 scene units," but
the user typically wants "1× the bounding sphere." Bounds-relative
zoom limits are missing.

```rust
// easy
OrbitControls::from_framing(framing).zoom_limits_bounds_relative(0.5, 4.0)
```

- [ ] `zoom_limits_bounds_relative(min_fraction, max_fraction)`.

### 3.3 `ConnectOptions::with_axial_gap`

The semantic is "axial gap of 0.4 along the mate axis." The API
should say so.

```rust
// today
options.with_mate_offset(Transform::at(Vec3::new(0.4, 0.0, 0.0)))

// easy
options.with_axial_gap(0.4)
options.with_clearance_mm(2.5)        // if input is millimeters
```

- [ ] `with_axial_gap(distance)`.
- [ ] `with_clearance_mm(mm)` for unit-explicit input.

### 3.4 `AnimationMixer::play_by_name`

Authored glTF animations have names. Looking up the key by name is
plumbing. Add a `play_by_name(&str)` shortcut.

```rust
// easy
mixer.play_by_name("idle")?;
mixer.play_by_name("door_open")?.with_loop(AnimationLoopMode::Once);
```

- [ ] `AnimationMixer::play_by_name(&str) -> Result<...>`.

### 3.5 Viewer pointer callbacks

`Viewer::pick_at(x, y)` exists. Users still wire pointer events
themselves. Add a callback-registration API for the common case.

```rust
// easy
viewer.on_click(|hit| println!("clicked {:?}", hit.node));
viewer.on_hover(|hit| ...);
```

- [ ] `Viewer::on_click(impl Fn(Hit))`, `on_hover`, `on_drag`.

### 3.6 Screenshot one-liner

The viewer has internal screenshot plumbing but the public ergonomic
isn't surfaced cleanly.

```rust
// easy
viewer.capture_png("frame.png")?;
viewer.capture_png_bytes()?;       // for upload pipelines
viewer.capture_gif("turntable.gif", Duration::from_secs(4))?;
```

- [ ] `Viewer::capture_png(path)`, `capture_png_bytes()`.
- [ ] (Stretch) `capture_gif`, `capture_mp4`.

### 3.7 Asset hot-reload during dev

Grep returned zero hits. drei's `useGLTF` invalidates on file change;
model-viewer reloads on `src` change.

- [ ] `Assets::with_hot_reload()` behind a `hot-reload` feature flag
      (native; WASM uses a different mechanism).

### 3.8 Drag-and-drop in the WASM viewer

`<scena-viewer>` (when it exists) should accept dropped files. Sister
to model-viewer's drop behavior.

- [ ] Drop-target handling on the WASM viewer canvas.

### 3.9 State-via-URL helper

`?camera-orbit=-28,18,2.5` round-tripping. model-viewer ships it.

- [ ] `FramingOutcome::serialize_url_state()` + matching deserializer.
- [ ] `OrbitControls` snapshot/restore for shareable URLs.

---

## 4. Shipping rounds

Rounds are sized to land independently. Each round closes a coherent
slice of "easier than Three.js" rather than spreading the work
horizontally.

### Round A — name, not number (signature)

1. - [ ] Named camera views + `azimuth_elevation` (§2.5)
2. - [ ] `Color` constants + `from_hex` + `from_kelvin` (§2.1)
3. - [ ] `PerspectiveCamera` lens presets + drop `with_aspect` (§2.2, §2.3)
4. - [ ] `Transform::rotated_*_degrees` + `looking_at` (§2.4)

### Round B — easy by name, continued

5. - [ ] Light presets (§2.6)
6. - [ ] `MaterialDesc` PBR presets (§2.7)
7. - [ ] `Background` enum (§2.8)
8. - [ ] `OrbitControls` named damping presets (§2.9)

### Round C — bundled content + feature shortcuts

9. - [ ] `Environment::*` with bundled KTX2 cubemaps
10. - [ ] `Assets::khronos::*` sample loaders (§3.1)
11. - [ ] `AutoExposureConfig` scenario presets (§2.10)
12. - [ ] `AnimationMixer::play_by_name` (§3.4)

### Round D — Tier 2 ergonomics

13. - [ ] `ConnectOptions::with_axial_gap` (§3.3)
14. - [ ] `OrbitControls` bounds-relative zoom (§3.2)
15. - [ ] `Viewer::on_click` / `on_hover` callbacks (§3.5)
16. - [ ] `Viewer::capture_png` and friends (§3.6)
17. - [ ] Asset hot-reload (§3.7)
18. - [ ] State-via-URL (§3.9)

---

## 5. Doctor enforcement pattern

For every Tier-1 named primitive that lands, add a doctor rule in the
same shape:

- [ ] Ban inline `Color::from_*(<float>, <float>, <float>)` arguments
      in `examples/` and `src/demo_page*` if a named `Color::*`
      constant or `Color::from_hex(...)` / `from_kelvin(...)` would do.
- [ ] Ban inline `with_fov(<float>)` if the value would match a lens
      preset.
- [ ] Ban inline `with_damping(<float>)` in `src/demo_page*` if a
      named damping preset would do.
- [ ] Ban inline `Quat::from_*(<float>, ...)` in `examples/` if
      `Transform::rotated_*_degrees` would do.
- [ ] Ban inline `look_from(Vec3::new(<lit>, <lit>, <lit>))` and
      `orbit(<lit>, <lit>)` in `src/demo_page*` (already in v1.3.0).

The rule: **wherever the library ships a name, the demo and example
code must use the name.** Magic numbers stay legal in user-application
code (escape hatches matter), but the library's own examples must
demonstrate the named surface — because the examples are the
documentation.

Rule shape lesson from v1.3.0: bind rules to the *residue pattern*,
not to dead API names. Inline-float-literal in a setter call is the
residue; the specific call's removed signature is not.

---

## 6. Renderer state-of-the-art gaps (separate arc)

Beyond ease-of-use, the renderer itself is missing table-stakes for
"state of the art in 2026." Listed for awareness; sequencing is
separate from Rounds A–D above.

- [ ] **Skinned / morph / clip-sampled animation playback.** Without
      `KHR_animations` playback, scena is a static model viewer, not a
      3D library. Renderer scope, not game-engine scope.
- [ ] **Anti-aliasing.** MSAA at minimum; TAA preferred.
- [ ] **Contact shadows / SSAO.** Single biggest "pro vs amateur" tell
      beyond framing. Without contact shadows the model floats on the
      grid regardless of how well it's framed.
- [ ] **Subtle bloom in post.** One low-threshold pass is the
      difference between "rendered" and "photographed."
- [ ] **Material coverage:** clearcoat, sheen, anisotropy, iridescence,
      dispersion on top of the existing metal-rough + transmission.
      The Khronos sample set assumes these.
- [ ] **Clustered / tiled light culling.** Babylon 9 made this
      baseline. Without it, scena is locked to a handful of lights.
- [ ] **KTX2 / Basis Universal textures (`KHR_texture_basisu`).** 4–8×
      GPU memory reduction. Required for twins of any meaningful size.
- [ ] **Mesh compression on import: Draco + meshoptimizer**
      (`KHR_draco_mesh_compression` + `EXT_meshopt_compression`).
- [ ] **GPU instancing import (`EXT_mesh_gpu_instancing`).** Mandatory
      for any twin with repeated parts.
- [ ] **Area lights with LTC (rect/disc/sphere).** Babylon 9 ships
      textured area lights; Filament has had them.
- [ ] **Screen-space reflections (SSR).** Floor reflections are now
      expected.
- [ ] **Order-independent transparency (OIT).** WaterBottle plus
      multiple transparent layers is what reviewers critique next.
      Weighted-blended OIT is the cheap baseline.
- [ ] **Wide-gamut output (Display P3).** Browser canvas supports
      `colorSpace: "display-p3"` for WebGL/WebGPU.
- [ ] **Khronos glTF Validator integration in `doctor`.** Sample
      Viewer 1.1 added a validator tab. scena's doctor tool is the
      natural home for per-asset validation with structured errors.

---

## 7. Ease-of-use signature opportunities

Higher-leverage than the renderer-quality work above. Each corresponds
to a specific competitor primitive known to delight users.

- [ ] **`<scena-viewer>` custom element with `<model-viewer>`
      attribute parity.** The single most important item.
      `<scena-viewer src="..." environment="studio" ar camera-controls 
      auto-rotate tone-mapping="neutral">` is the difference between
      "a Rust crate" and "the thing people reach for when they need to
      put a model on a page."
- [ ] **Bundled studio environments as a Rust enum.** drei's
      `<Environment preset="studio|city|sunset|...">` is the gold
      standard. Embed 6–8 small KTX2 cubemaps in the crate.
- [ ] **Camera control kit.** Minimum: Orbit, Turntable/Presentation,
      Follow, Fly, one-call "cinematic" preset.
- [ ] **Picking + outline + hover.** drei `<Select>`, `useCursor`,
      three.js `OutlinePass`. For inspection workflows this is
      mandatory: click a part, get a glow.
- [ ] **HTML/CSS annotation overlay anchored to 3D points.**
      model-viewer's `data-position` / `data-normal` / `data-surface`.
      The `data-surface` trick (label sticks to a deforming surface)
      is the killer feature.
- [ ] **Variant switching for `KHR_materials_variants`.** "Same
      chassis, different SKU."
- [ ] **Loading progress primitives.** drei `<Loader>` + `useProgress`;
      model-viewer `poster` + `reveal="interaction"`.
- [ ] **Actionable error messages.** Khronos validator error codes plus
      Rust enum errors with `fix` hints:
      ```rust
      SceneLoadError::TextureFormatUnsupported {
          texture: "albedo.webp",
          reason: "WebP animation not allowed by glTF",
          fix: "Re-export with PNG or use KTX2",
      }
      ```
      Rust enums + scena's doctor combine into something genuinely
      better than the JS competitors here.
- [ ] **Mobile-first + a11y defaults.** Touch, pinch-zoom, adaptive
      resolution, arrow-key rotation, ARIA live region for camera
      state, `alt` text.
- [ ] **Inspector / dev overlay.** Single overlay (FPS + draw calls +
      GPU adapter + capability warnings + tonemap toggle + exposure
      slider + environment picker + wireframe/normals/UVs/AO) toggled
      by a keyboard shortcut. doctor is already half of this.

---

## 8. Differentiators scena could uniquely own

No competitor has these. Each builds on something scena already has.

- [ ] **Connector "magnet" snapping with visual cues.** Ghost + green
      outline when a valid mate is within tolerance. Builds on the
      connector mating work. No general-purpose library does this.
      *Triggered when an interactive drag-to-assemble workflow has a
      concrete consumer; not needed for read-only viewing.*
- [ ] **CPU rasterizer fallback in WASM for server-side screenshots.**
      scena already has a CPU path. No JS library does this — for
      OG-image generation / server-side preview rendering, this is a
      real moat.
- [ ] **Reference-image regression as a public API.** scena already
      does this internally (`SCENA_REFERENCE_DIFF` against the Khronos
      WaterBottle reference). Expose `scena::regress(asset, expected)`
      for end users.

---

## 9. Explicit non-goals

Skip these — they're game-engine / simulation territory and would
dilute scena's renderer-only positioning:

- Physics, collision detection, rigid bodies (Rapier territory).
- Game loop / ECS as a public API (Bevy territory).
- Audio, positional audio.
- Particle systems beyond simple sprites.
- AI navigation, pathfinding, character controllers.
- Networking / multiplayer state sync.
- Geometry-creation asset editor (keep import-only).
- Visual node editor / scripting language for materials.
- Animation *authoring* (import + playback is renderer; authoring is
  not).

---

## 10. The four bets that move scena to state-of-the-art

If only four big things can be funded in addition to Rounds A–D:

1. **`<scena-viewer>` custom element with model-viewer attribute
   parity.** The difference between "a Rust crate" and "the thing
   people reach for."
2. **`frame_bounds` as default + named studio environments as a Rust
   enum.** Drop-a-model-and-it-looks-good without 12 knobs to tune.
3. **PBR Neutral default tonemapper + KTX2 + Draco + meshopt + GPU
   instancing on import.** The compression/quality features that take
   scena from "Rust toy" to "deploy this to production."
4. **Extend `doctor` into per-asset validation + actionable Rust enum
   errors with `fix` hints.** doctor is already most of an inspector;
   finishing it gives scena the single most actionable error story of
   any 3D library.

Together those four make "easier than Three.js, more accurate than
model-viewer" a defensible claim.

---

## 11. Positioning verdict

For scena's actual positioning — a Rust renderer for trust-platform /
digital-twin applications — Rounds A–D + bets 1–4 above get scena to
**credibly competitive with Three.js for static product viewing**.
That's a defensible "state-of-the-art static product / digital-twin
viewer" claim.

The unqualified **"state-of-the-art 3D library"** claim needs at least
animation playback (§6), contact shadows (§6), and anti-aliasing (§6)
before it survives someone running the same glTF through
`<model-viewer>` or Three.js side by side. This roadmap is a
necessary step, not a sufficient one — the follow-up arc that closes
the rest of the gap is animation + AA + contact shadows + bloom +
material coverage.

# Pre-render lint profile design

Status: deferred design for a future release; no `scena lint` command is
claimed by v1.9.0.

`scena lint` is intentionally specified as a composition profile, not a new
validation engine. A future implementation must call the existing schema
dispatcher, recipe validator and resolver, capability planner, asset doctor,
animation/camera validators, and scene visibility diagnostics. It must not copy
their rules or turn rendering into a hidden validation side effect.

The proposed stable result will use a versioned `scena.lint_result.<version>`
schema with `ok`, `profile`,
`checks`, `findings`, and `evidence` members. Every finding carries its owning
subsystem's stable code, severity, message, context/path, help, candidates, and
optional structured fix. The command exits 0 only when no error finding exists;
invalid invocation uses the normal CLI usage taxonomy, and a completed lint
with findings returns the input-error class without replacing the result
envelope with prose.

Two profiles are required:

- `offline` performs JSON/schema validation, complete recipe resource
  resolution under the effective sandbox policy, static camera and animation
  validation, unsupported-feature planning against an explicitly selected
  backend tier, and deterministic scene diagnostics. It covers missing camera
  or lighting, invisible scene state, unresolved assets, policy denial,
  invalid animation/camera data, and statically unsupported features.
- `live` adds an adapter/device capability probe and backend-dependent resource
  checks. It does not render a frame. Adapter absence is a structured error,
  never an offline pass.

The `evidence` array records every component as `passed`, `failed`, or
`skipped`, with owner, mode, and reason. A skipped check can never satisfy a
required profile. Offline mode must explicitly mark live-only evidence skipped;
live mode must fail closed when a required probe cannot execute. The output
also records the effective recipe policy and capability source so callers can
distinguish declared planning from measured hardware.

Before implementation, add a failing CLI fixture containing an unresolved
asset, missing camera/lighting, invalid animation data, and an unsupported
feature, and prove that one result reports all independently discoverable
findings. Add parity tests showing each finding matches its owner subsystem and
a mutation test proving a required skipped check cannot produce `ok:true`.

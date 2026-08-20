# scena 1.10.0 release review

Reviewed: 2026-08-20

Candidate scope: the completed 1.9.1 remediation batch plus the additive
subject-driven photographic-rendering, materials, camera-projection, WebGL2,
recipe-framing, and section-cut work rebased onto `main`.

## Release decision

The candidate is a minor release because it adds public CLI commands,
versioned JSON contracts, recipe fields, and Rust/SceneHost API. Existing
recipes retain their explicit camera, exposure, focus, and average-metering
behavior unless they opt into the new photo intent.

The candidate is not a published release until its exact commit passes the
release matrix and the `v1.10.0` tag is created from the merged `main` commit.

## Evidence boundary

The release validation must distinguish CPU/builder, browser software
conformance, and real-hardware proof. Mesa/V3D investigation results are not
part of this release and make no performance claim here.

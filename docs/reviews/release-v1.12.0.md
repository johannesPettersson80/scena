# scena 1.12.0 release review

This minor release maps equirectangular environments as three.js does and
subdivides glossy CPU-shaded triangles. Rendered output changes for scenes lit
by an equirectangular HDR and for smooth untextured materials on the CPU
renderer, so the release is accepted only when the CPU reference proofs, the
Round E material proofs, the CI matrix and the Release workflow pass on the
tagged commit, and the public tag, GitHub release, crates.io archive, and
docs.rs agree on one source commit.

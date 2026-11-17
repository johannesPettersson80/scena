# scena 1.11.0 release review

This minor release adds path-traced stills, a layered surface model shared by
the CPU and GPU renderers, exact world-matrix placement, and hardened glTF
import. Rendered output changes for layered materials and several GPU
post-processing passes, so the release is accepted only when the CPU reference
proofs, the CI matrix and the Release workflow pass on the tagged commit, and
the public tag, GitHub release, crates.io archive, and docs.rs agree on one
source commit.

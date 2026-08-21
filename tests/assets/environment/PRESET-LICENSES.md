# Bundled environment preset licenses

- `neutral-studio.fixture.txt` and its generated derivatives are scena-owned
  deterministic fixtures distributed under CC0-1.0.
- `polyhaven/studio_small_08_1k.hdr` is the direct 1K `studio_small_08`
  environment from Poly Haven, distributed under CC0-1.0. Source:
  <https://polyhaven.com/a/studio_small_08>. Its SHA-256 is
  `f6a989f89432eb4eee3191364a9c1ceed195c4ec3544173a3c04fd96cb91d0ba`.
  The 2K upstream distribution is not bundled because its malformed RLE stream
  is rejected by the supported Rust decoders; the 1K source preserves the
  approved 512-pixel-cubemap lighting result.
  `generated/studio_small_03_128x64.hdr` is the package-embedded runtime
  derivative, retained for the interactive preset profile.
  source. Its SHA-256 is
  `0d1acad01f8d664bb64072af7423f6c133fa57dadd795d5278b256c99eee0bd6`.

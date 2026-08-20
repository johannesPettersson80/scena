# scena 1.10.0 release checklist

Release candidate created: 2026-08-20

## Scope

- Completed 1.9.1 remediation work.
- Subject-driven photographic rendering and reports.
- Offline material catalog/import workflow.
- Camera projection, recipe framing, WebGL2 presentation, and closed-mesh
  section-cut fixes.

## Required release evidence

- Remote builder formatting, clippy, tests, doctor, documentation, and publish
  dry-run on the exact candidate SHA.
- One GitHub CI run for the candidate branch; collect all failures before any
  corrective push.
- Merge the passing candidate to `main`, then create and verify `v1.10.0`.

## Excluded scope

- Mesa/V3D compiler experiments and performance claims.
- Any unpublished hardware claim not produced by the configured release lanes.

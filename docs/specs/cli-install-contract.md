# CLI installation contract

The selected v1.x contract keeps the library's Cargo default feature set empty.
`cargo install scena` installs the core discovery, schema, vocabulary, policy,
validation, capability, and conversion commands. It does not claim to render,
inspect, diagnose, repair, or generate agent templates. Those application-builder
commands require the one-step `agent` composition:

```bash
cargo install scena --features agent
```

This is deliberate:

- making `agent` a package default would also compile scene-host and inspection
  into every ordinary library dependency, increasing build time and binary
  surface for users who only need the renderer library;
- Cargo cannot enable a package feature only for one binary in the same package,
  so “always enabled for the CLI only” is not a real single-package option;
- a separate `scena-cli` package would isolate those costs but adds a second
  publication, version, install name, security surface, and compatibility
  matrix. It remains an option only if measured install demand justifies it.

Every feature-unavailable command returns `scena.cli_error.v1` with
`code:"feature_unavailable"`, `exit_class:"unsupported"`, exit 69, and exactly
one install remedy naming `--features agent`. Packaged-crate tests install both
the default and agent variants in clean roots. The default smoke proves core
commands plus structured unavailable results; the agent smoke executes help,
schema discovery, validation, template discovery, render, inspect, diagnose,
doctor, and repair outside a repository checkout.

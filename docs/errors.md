# Errors and diagnostics

`scena` uses structured errors so applications can recover predictably and show
useful messages.

## Error families

| Error | Typical cause |
|---|---|
| `BuildError` | renderer or platform construction failed |
| `AssetError` | asset loading, fetching, decoding, or extension handling failed |
| `ImportError` | imported scene data could not be interpreted |
| `InstantiateError` | asset instantiation into a scene failed |
| `LookupError` | a named node, path, anchor, connector, or handle lookup failed |
| `PrepareError` | renderer preparation failed |
| `RenderError` | rendering failed, often because prepared state is stale |
| `AnimationError` | clip, mixer, channel, skin, or morph target operation failed |
| `ConnectionError` | anchor or connector placement failed |
| `ColorParseError` | color parsing failed |

## Pattern matching

Use Rust pattern matching for application logic:

```rust
match renderer.render_active(&scene) {
    Ok(frame) => frame,
    Err(err) => {
        eprintln!("{err}");
        return Err(err.into());
    }
}
```

Use richer matching when an application needs specific recovery behavior, such
as preparing again after stale renderer state.

## Common recoveries

| Problem | Recovery |
|---|---|
| Render called before prepare | call `prepare()` and render again |
| Scene changed after prepare | call `prepare()` again |
| Surface resized | forward the surface event, then prepare again |
| Missing asset file | fix path or fetcher configuration |
| Unsupported required glTF extension | enable the relevant feature or choose an asset variant without that required extension |
| Missing named node or anchor | inspect imported names and paths |
| Browser backend unavailable | choose another backend or show a capability message |

## Diagnostics

Renderer diagnostics and capability reports are designed for user-facing error
messages and bug reports. Include them when reporting platform-specific issues.

Useful diagnostic information:

- backend,
- adapter name,
- active feature flags,
- asset path,
- glTF extension name,
- scene/import handle,
- renderer capability report,
- renderer stats.

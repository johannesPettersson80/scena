# Authoring glTF Anchors And Connectors

Type: Guide.

`extras.scena` stores renderer-level placement metadata. It must not contain process,
physics, robotics, PLC, or simulation semantics.

## Anchors

```json
{
  "extras": {
    "scena": {
      "anchors": [
        {
          "name": "inspection",
          "translation": [0.0, 0.1, 0.0],
          "forward": [1.0, 0.0, 0.0],
          "up": [0.0, 1.0, 0.0],
          "tags": ["service"],
          "label": "Inspection Port"
        }
      ]
    }
  }
}
```

Anchors are passive. Adapt them with `ConnectorFrame::from_import_anchor` when they should
drive placement.

An anchor transform uses exactly one orientation form:

- normalized quaternion `rotation: [x, y, z, w]`;
- paired, finite, nonzero, nonparallel `forward` and `up` vectors; or
- a finite column-major 16-value affine `matrix` that decomposes exactly into
  translation, rotation, and nonzero scale without shear.

`translation` and `scale` are three finite numbers, and every scale component
must be nonzero. A `matrix` cannot be combined with TRS or forward/up fields.
Supplying only one of `forward` or `up` is an authoring error.

## Connectors

```json
{
  "name": "mount",
  "kind": "mount",
  "translation": [0.0, 0.1, 0.0],
  "allowedMates": ["socket"],
  "tags": ["assembly"],
  "snapTolerance": 0.025,
  "clearanceHint": 0.01,
  "rollPolicy": "chooseNearest",
  "polarity": "plug",
  "metadata": { "author": "fixture" }
}
```

Connectors are connection-capable anchors. Use `Scene::connect_import_connectors` for the
short path when both imported parts expose stable connector names.

Connectors use the same strict transform grammar as anchors. For example, this
matrix form is equivalent to an identity orientation translated 0.1 source
units along Y:

```json
{
  "name": "matrix-mount",
  "kind": "mount",
  "matrix": [
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    0.0, 0.1, 0.0, 1.0
  ]
}
```

Invalid marker transforms abort glTF loading with `AssetError::Parse`. The
diagnostic names the asset and exact JSON path, for example
`nodes[0].extras.scena.connectors[0].up`; invalid authored orientation is never
replaced with identity. Marker-unit overrides and import coordinate conversion
are applied only after this validation succeeds.

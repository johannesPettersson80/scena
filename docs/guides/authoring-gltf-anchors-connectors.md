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

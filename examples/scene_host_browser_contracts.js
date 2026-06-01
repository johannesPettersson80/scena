import init, { SceneHost } from "../pkg/scena.js";

export async function buildSceneHost(canvas, leftUrl, rightUrl) {
  await init();

  const width = canvas.clientWidth || 640;
  const height = canvas.clientHeight || 480;
  const dpr = window.devicePixelRatio || 1;
  const host = await SceneHost.newWebgl2(canvas, width, height, dpr);
  const root = host.rootHandle();

  const leftFrame = host.addEmpty(
    root,
    [-0.6, 0.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
    [1.0, 1.0, 1.0],
    "frame:left",
  );
  const rightFrame = host.addEmpty(
    root,
    [0.6, 0.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
    [1.0, 1.0, 1.0],
    "frame:right",
  );

  const leftImportJson = await host.instantiateUrlUnderWithReportJson(leftFrame, leftUrl);
  const leftImport = JSON.parse(leftImportJson).import;
  const rightImport = await host.instantiateUrlUnder(rightFrame, rightUrl);
  const leftMesh = host.nodeHandle(leftImport, "ColoredTriangle");
  const rightMesh = host.nodeHandleByName(rightImport, "ColoredTriangle");

  return { host, leftFrame, rightFrame, leftMesh, rightMesh, leftImportJson };
}

export async function renderPushedFrame(state, poseByNode) {
  state.host.setTransforms(JSON.stringify(
    poseByNode.map(([node, transform]) => ({
      node,
      translation: transform.translation,
      rotation: transform.rotation,
      scale: transform.scale,
    })),
  ));
  state.host.setNodeAnnotation("left-label", state.leftMesh, [0.0, 0.0, 0.0]);
  state.host.frameAll();
  state.host.prepare();
  state.host.render();

  const capture = state.host.capture();
  return {
    inspection: JSON.parse(state.host.inspectJson()),
    capture: JSON.parse(capture.descriptorJson),
    annotationProjection: JSON.parse(state.host.annotationProjectionsJson()),
    rgba8: capture.rgba8,
  };
}

export function pickCssPixel(state, event) {
  return state.host.pick(event.offsetX, event.offsetY);
}

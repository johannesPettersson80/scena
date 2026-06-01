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

export function wireSceneHostCamera(canvas, state, requestFrame) {
  let lastPointer = null;
  const request = (action) => {
    if (action !== "none" && action !== "begin_orbit" && action !== "end") {
      requestFrame();
    }
  };
  const buttonName = (button) => {
    if (button === 2) {
      return "secondary";
    }
    if (button === 1) {
      return "auxiliary";
    }
    return "primary";
  };

  canvas.addEventListener("pointerdown", (event) => {
    event.preventDefault();
    lastPointer = { x: event.clientX, y: event.clientY };
    canvas.setPointerCapture?.(event.pointerId);
    request(state.host.cameraPointerDown(event.clientX, event.clientY, buttonName(event.button)));
  });

  canvas.addEventListener("pointermove", (event) => {
    if (!lastPointer) {
      return;
    }
    event.preventDefault();
    const dx = event.clientX - lastPointer.x;
    const dy = event.clientY - lastPointer.y;
    lastPointer = { x: event.clientX, y: event.clientY };
    request(state.host.cameraPointerMove(event.clientX, event.clientY, dx, dy));
  });

  const endPointer = (event) => {
    if (!lastPointer) {
      return;
    }
    event.preventDefault();
    lastPointer = null;
    request(state.host.cameraPointerUp(event.clientX, event.clientY));
  };
  canvas.addEventListener("pointerup", endPointer);
  canvas.addEventListener("pointercancel", endPointer);

  canvas.addEventListener("wheel", (event) => {
    event.preventDefault();
    request(state.host.cameraWheel(event.offsetX, event.offsetY, event.deltaY));
  }, { passive: false });
}

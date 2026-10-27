/**
 * Sigma 3 opens three WebGL contexts (edges, nodes, hover) and then calls
 * `gl.blendFunc` with no null check. A browser that is out of contexts returns
 * null from `getContext`, and the constructor throws
 * `Cannot read properties of null (reading 'blendFunc')`.
 * A throw midway also leaves the canvases it already created attached, so the
 * next attempt has even fewer slots.
 */

type GL = WebGLRenderingContext | WebGL2RenderingContext;

function takeContext(canvas: HTMLCanvasElement): GL | null {
  return (
    canvas.getContext("webgl2") ||
    canvas.getContext("webgl") ||
    (canvas.getContext("experimental-webgl") as GL | null)
  );
}

function lose(gl: GL): void {
  gl.getExtension("WEBGL_lose_context")?.loseContext();
}

/** True when `count` WebGL contexts can be created right now. The probe is released. */
export function webglSlotsFree(count = 3): boolean {
  if (typeof document === "undefined") return false;
  const held: GL[] = [];
  for (let i = 0; i < count; i += 1) {
    const gl = takeContext(document.createElement("canvas"));
    if (!gl) {
      held.forEach(lose);
      return false;
    }
    held.push(gl);
  }
  held.forEach(lose);
  return true;
}

/** Drop canvases a failed Sigma constructor left in the container, and free their contexts. */
export function releaseContainerWebGL(container: HTMLElement): void {
  container.querySelectorAll("canvas").forEach((node) => {
    const canvas = node as HTMLCanvasElement;
    const gl = takeContext(canvas);
    if (gl) lose(gl);
    canvas.remove();
  });
}

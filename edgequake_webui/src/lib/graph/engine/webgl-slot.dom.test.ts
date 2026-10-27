import { afterEach, describe, expect, it, vi } from "vitest";
import { releaseContainerWebGL, webglSlotsFree } from "./webgl-slot";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("webgl slot", () => {
  it("allocates the three contexts Sigma opens, then releases them", () => {
    const loseContext = vi.fn();
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(
      () => ({ getExtension: () => ({ loseContext }) }) as never,
    );
    expect(webglSlotsFree()).toBe(true);
    expect(loseContext).toHaveBeenCalledTimes(3);
  });

  it("refuses Sigma when getContext returns null", () => {
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null);
    expect(webglSlotsFree()).toBe(false);
  });

  it("removes a canvas a failed constructor left in the container", () => {
    const loseContext = vi.fn();
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(
      () => ({ getExtension: () => ({ loseContext }) }) as never,
    );
    const host = document.createElement("div");
    host.appendChild(document.createElement("canvas"));
    releaseContainerWebGL(host);
    expect(host.querySelector("canvas")).toBeNull();
    expect(loseContext).toHaveBeenCalledOnce();
  });
});

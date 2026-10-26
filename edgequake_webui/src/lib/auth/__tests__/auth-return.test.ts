import { describe, expect, it, vi } from "vitest";
import { followAuthRedirect, isOAuthAuthorizeReturn } from "../auth-return";

describe("auth return after MCP login", () => {
  it("treats the OAuth authorize path as a document navigation", () => {
    expect(isOAuthAuthorizeReturn("/oauth/authorize?response_type=code&redirect_uri=https%3A%2F%2Fclient.example%2Fcb")).toBe(true);
    expect(isOAuthAuthorizeReturn("/documents")).toBe(false);
    expect(isOAuthAuthorizeReturn("/oauth-evil")).toBe(false);
    expect(isOAuthAuthorizeReturn("/oauth/../login")).toBe(false);
  });

  it("assigns the OAuth return and pushes every other path", () => {
    const navigate = vi.fn();
    const assign = vi.fn();
    followAuthRedirect(navigate, "/oauth/authorize?state=s1", assign);
    expect(assign).toHaveBeenCalledWith("/oauth/authorize?state=s1");
    expect(navigate).not.toHaveBeenCalled();

    followAuthRedirect(navigate, "/documents", assign);
    expect(navigate).toHaveBeenCalledWith("/documents");
  });
});

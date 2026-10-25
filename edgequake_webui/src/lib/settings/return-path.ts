/**
 * SPEC-175 — "Back to app" returns to the last non-settings page the user visited this
 * session. Stored in sessionStorage, so it never leaks across tabs or sessions.
 */

const KEY = "edgequake:settings-return-path";

/** True for `/settings...` and `/w/{slug}/settings...`. */
export function isSettingsPath(pathname: string): boolean {
  return /^\/settings(\/|$)/.test(pathname) || /^\/w\/[^/]+\/settings(\/|$)/.test(pathname);
}

/** Records an app path unless it is a settings path. Safe to call during render-free effects only. */
export function rememberAppPath(pathname: string): void {
  if (typeof window === "undefined" || isSettingsPath(pathname)) return;
  try {
    window.sessionStorage.setItem(KEY, pathname);
  } catch {
    // Storage can be blocked (private mode). "Back to app" then falls back to "/".
  }
}

export function rememberedAppPath(): string | null {
  if (typeof window === "undefined") return null;
  try {
    return window.sessionStorage.getItem(KEY);
  } catch {
    return null;
  }
}

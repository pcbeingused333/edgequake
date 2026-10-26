/**
 * Where to go after password login or SSO handoff.
 *
 * `/oauth/authorize` is served by the API (Caddy/ingress), not by Next.js.
 * A client-side `router.push` never establishes the OAuth session and the
 * API sends the browser back to the login form. Those returns are a full
 * document navigation so the HttpOnly `eq_oauth_session` cookie is sent.
 */

/** True for the MCP authorize return (`/oauth` or `/oauth/…`). */
export function isOAuthAuthorizeReturn(path: string): boolean {
  const pathOnly = path.split("#")[0]?.split("?")[0] ?? path;
  if (pathOnly !== "/oauth" && !pathOnly.startsWith("/oauth/")) return false;
  if (pathOnly.includes("..") || pathOnly.includes("//")) return false;
  return true;
}

/**
 * Continue after sign-in. `navigate` is `router.push` or `router.replace`.
 * `assign` is injectable so tests do not touch `window.location`.
 */
export function followAuthRedirect(
  navigate: (href: string) => void,
  path: string,
  assign: (href: string) => void = (href) => window.location.assign(href),
): void {
  if (isOAuthAuthorizeReturn(path)) {
    assign(path);
    return;
  }
  navigate(path);
}

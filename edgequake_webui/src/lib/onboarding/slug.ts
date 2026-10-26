/**
 * Mirror of the server's `generate_slug` (handlers/workspaces/helpers.rs):
 * lower-case, every non-alphanumeric run becomes one `-`, trimmed.
 * Lets the wizard preview the URL slug the server will assign.
 */
export function slugifyName(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, '-')
    .replace(/^-+|-+$/g, '');
}

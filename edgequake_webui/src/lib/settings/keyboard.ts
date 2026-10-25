/**
 * SPEC-175 — pure keyboard math for the settings nav (roving links) and search
 * (combobox options). Kept free of React so the rules are unit-tested (EC-175-41).
 */

/** Index of the next nav link for a key, or null when the key is not a nav key. */
export function nextNavIndex(current: number, count: number, key: string): number | null {
  if (count === 0) return null;
  switch (key) {
    case "ArrowDown":
      return current < 0 ? 0 : (current + 1) % count;
    case "ArrowUp":
      return current < 0 ? count - 1 : (current - 1 + count) % count;
    case "Home":
      return 0;
    case "End":
      return count - 1;
    default:
      return null;
  }
}

/** Active option index after an arrow key, clamped (no wrap) to the result list. */
export function nextActiveOption(current: number, count: number, key: string): number {
  if (count === 0) return -1;
  if (key === "ArrowDown") return current < 0 ? 0 : Math.min(current + 1, count - 1);
  if (key === "ArrowUp") return current <= 0 ? count - 1 : current - 1;
  return current;
}

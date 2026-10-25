import { describe, expect, it } from "vitest";
import { nextActiveOption, nextNavIndex } from "../keyboard";

describe("nextNavIndex (roving nav, WCAG 2.1.1)", () => {
  it("ArrowDown and ArrowUp wrap around", () => {
    expect(nextNavIndex(2, 3, "ArrowDown")).toBe(0);
    expect(nextNavIndex(0, 3, "ArrowUp")).toBe(2);
  });

  it("starts at the first or last link when nothing is focused", () => {
    expect(nextNavIndex(-1, 3, "ArrowDown")).toBe(0);
    expect(nextNavIndex(-1, 3, "ArrowUp")).toBe(2);
  });

  it("Home and End jump to the ends", () => {
    expect(nextNavIndex(1, 4, "Home")).toBe(0);
    expect(nextNavIndex(1, 4, "End")).toBe(3);
  });

  it("returns null for keys the nav does not handle and for an empty list", () => {
    expect(nextNavIndex(0, 3, "Enter")).toBeNull();
    expect(nextNavIndex(0, 0, "ArrowDown")).toBeNull();
  });
});

describe("nextActiveOption (combobox, clamped)", () => {
  it("moves down and clamps at the last option", () => {
    expect(nextActiveOption(-1, 3, "ArrowDown")).toBe(0);
    expect(nextActiveOption(2, 3, "ArrowDown")).toBe(2);
  });

  it("moves up and clamps at the first option; from none it goes to the last", () => {
    expect(nextActiveOption(1, 3, "ArrowUp")).toBe(0);
    expect(nextActiveOption(0, 3, "ArrowUp")).toBe(2);
    expect(nextActiveOption(-1, 3, "ArrowUp")).toBe(2);
  });

  it("with no results there is no active option", () => {
    expect(nextActiveOption(0, 0, "ArrowDown")).toBe(-1);
  });
});

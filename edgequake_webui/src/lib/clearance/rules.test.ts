import { describe, expect, it } from "vitest";
import { draftFromRules, rulesFromDraft, sameDraft } from "./rules";

describe("rulesFromDraft", () => {
  it("maps a ceiling and an except set", () => {
    expect(
      rulesFromDraft({
        ceiling: "confidential",
        departmentIds: ["hr"],
        exceptIds: ["atlas"],
      }),
    ).toEqual([
      { effect: "allow", match_kind: "classification_at_most", rank: 3 },
      { effect: "allow", match_kind: "any_of", tag_ids: ["hr"] },
      { effect: "deny", match_kind: "any_of", tag_ids: ["atlas"] },
    ]);
  });
});

describe("draftFromRules", () => {
  it("reads a saved profile back into the same draft", () => {
    const draft = {
      ceiling: "restricted" as const,
      departmentIds: ["hr", "rnd"],
      exceptIds: ["atlas"],
    };
    const saved = rulesFromDraft(draft).map((rule) => ({
      rank: null,
      tag_ids: [],
      ...rule,
    }));
    expect(draftFromRules(saved)).toEqual(draft);
  });

  it("reads an unverified rank as the nearest editable ceiling", () => {
    const draft = draftFromRules([
      { effect: "allow", match_kind: "classification_at_most", rank: 5, tag_ids: [] },
    ]);
    expect(draft.ceiling).toBe("restricted");
  });

  it("falls back to Internal when no ceiling rule is saved", () => {
    expect(draftFromRules([])).toEqual({
      ceiling: "internal",
      departmentIds: [],
      exceptIds: [],
    });
  });

  it("merges department ids from several allow rules without duplicates", () => {
    const draft = draftFromRules([
      { effect: "allow", match_kind: "any_of", rank: null, tag_ids: ["hr", "legal"] },
      { effect: "allow", match_kind: "any_of", rank: null, tag_ids: ["hr"] },
    ]);
    expect(draft.departmentIds.sort()).toEqual(["hr", "legal"]);
  });
});

describe("sameDraft", () => {
  it("ignores the order of department ids", () => {
    expect(
      sameDraft(
        { ceiling: "public", departmentIds: ["a", "b"], exceptIds: [] },
        { ceiling: "public", departmentIds: ["b", "a"], exceptIds: [] },
      ),
    ).toBe(true);
  });

  it("detects a changed ceiling", () => {
    expect(
      sameDraft(
        { ceiling: "public", departmentIds: [], exceptIds: [] },
        { ceiling: "internal", departmentIds: [], exceptIds: [] },
      ),
    ).toBe(false);
  });
});

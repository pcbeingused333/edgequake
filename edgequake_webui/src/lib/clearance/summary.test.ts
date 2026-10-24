import { describe, expect, it } from "vitest";
import {
  classificationWord,
  documentCountLabel,
  documentLabelSummary,
  profileLine,
  profileSummary,
  tagNames,
} from "./summary";
import type { ClearanceTag, SecurityRoleView } from "./types";

const tags: ClearanceTag[] = [
  { id: "hr", kind: "compartment", key: "hr", value: "HR", rank: null },
  { id: "rnd", kind: "compartment", key: "rnd", value: "R&D", rank: null },
  { id: "atlas", kind: "compartment", key: "atlas", value: "Atlas", rank: null },
];

describe("profileSummary", () => {
  it("joins ceiling, departments and exceptions with middle dots", () => {
    expect(
      profileSummary(
        { ceiling: "confidential", departmentIds: ["hr", "rnd"], exceptIds: ["atlas"] },
        tagNames(tags),
      ),
    ).toBe("Confidential · HR, R&D · except Atlas");
  });

  it("skips departments that no longer exist", () => {
    expect(
      profileSummary({ ceiling: "internal", departmentIds: ["gone"], exceptIds: [] }, tagNames(tags)),
    ).toBe("Internal");
  });
});

describe("profileLine", () => {
  it("says there is no access before any rule is saved", () => {
    const role: SecurityRoleView = { id: "r", name: "New", steward: false, rules: [] };
    expect(profileLine(role, tags)).toBe("No access yet");
  });
});

describe("classificationWord", () => {
  it("shows stewards that a document is not reviewed", () => {
    expect(classificationWord("unverified", true)).toBe("Not reviewed");
  });

  it("tells other members the document is only theirs until review", () => {
    expect(classificationWord("unverified", false)).toBe("Only you, until reviewed");
  });

  it("capitalises a plain ceiling", () => {
    expect(classificationWord("internal", false)).toBe("Internal");
  });
});

describe("documentLabelSummary", () => {
  it("combines the classification and compartment names", () => {
    expect(documentLabelSummary("internal", ["HR"], false)).toBe("Internal · HR");
  });

  it("shows the classification alone when there are no compartments", () => {
    expect(documentLabelSummary("public", [], false)).toBe("Public");
  });
});

describe("documentCountLabel", () => {
  it("uses singular for one document", () => {
    expect(documentCountLabel(1)).toBe("1 document");
  });

  it("uses plural for several", () => {
    expect(documentCountLabel(3)).toBe("3 documents");
  });

  it("names the cap when the count is unknown", () => {
    expect(documentCountLabel(null)).toBe("More than 20 documents");
  });
});

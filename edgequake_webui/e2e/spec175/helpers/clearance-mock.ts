/**
 * SPEC-175 — hermetic document-access (clearance) server for the Access screen.
 *
 * Serves the three reads the Access card makes (settings, vocabulary, profiles) with a realistic
 * workspace: two departments, two working profiles and one profile with no access yet. Writes
 * return success so the editors can be exercised without a live backend.
 */
import type { Page, Route } from "@playwright/test";
import { SPEC155_WORKSPACE } from "../../spec155/helpers/mock-api";

export interface ClearanceScenario {
  /** Whether the caller may manage this workspace's document access. */
  canManage: boolean;
  /** Whether document labels are already turned on. */
  enforcement?: boolean;
  /** Make the settings read fail, to exercise the error state. */
  failSettings?: boolean;
}

export const SPEC175_TAGS = [
  { id: "tag-finance", kind: "compartment", key: "finance", value: "Finance", rank: null },
  { id: "tag-engineering", kind: "compartment", key: "engineering", value: "Engineering", rank: null },
  { id: "tag-legal", kind: "compartment", key: "legal", value: "Legal", rank: null },
];

export const SPEC175_PROFILES = [
  {
    id: "role-finance-analysts",
    name: "Finance analysts",
    steward: false,
    rules: [
      { effect: "allow", match_kind: "classification_at_most", rank: 2, tag_ids: [] },
      { effect: "allow", match_kind: "any_of", rank: null, tag_ids: ["tag-finance"] },
    ],
  },
  {
    id: "role-engineering",
    name: "Engineering",
    steward: false,
    rules: [
      { effect: "allow", match_kind: "classification_at_most", rank: 1, tag_ids: [] },
      { effect: "allow", match_kind: "any_of", rank: null, tag_ids: ["tag-engineering"] },
    ],
  },
  { id: "role-contractors", name: "Contractors", steward: false, rules: [] },
];

const CLEARANCE_PATH = `/api/v1/workspaces/${SPEC155_WORKSPACE.id}/clearance`;

function json(route: Route, body: unknown, status = 200) {
  return route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
}

/** Routes the clearance reads and accepts writes. Registered after the hermetic guard. */
export async function mockClearance(page: Page, scenario: ClearanceScenario): Promise<void> {
  const settings = {
    enforcement: scenario.enforcement ?? false,
    policy_epoch: 3,
    steward: false,
    rank: "admin",
    can_manage: scenario.canManage,
  };

  await page.route(new RegExp(`${CLEARANCE_PATH}(\\?.*)?$`), (route) => {
    if (route.request().method() !== "GET") return json(route, { enforcement: settings.enforcement });
    if (scenario.failSettings) return json(route, { error: "clearance unavailable" }, 500);
    return json(route, settings);
  });
  await page.route(new RegExp(`${CLEARANCE_PATH}/tags(\\?.*)?$`), (route) =>
    route.request().method() === "GET" ? json(route, SPEC175_TAGS) : json(route, { ok: true }),
  );
  await page.route(new RegExp(`${CLEARANCE_PATH}/roles(\\?.*)?$`), (route) =>
    route.request().method() === "GET" ? json(route, SPEC175_PROFILES) : json(route, { ok: true }),
  );
  await page.route(new RegExp(`${CLEARANCE_PATH}/simulate(\\?.*)?$`), (route) =>
    json(route, { titles: ["Q3 budget.pdf", "Vendor contracts.pdf"], count: 2 }),
  );
}

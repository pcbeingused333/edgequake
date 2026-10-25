/**
 * SPEC-175 — hermetic `GET /api/v1/me/settings-access` for each role. The server is the
 * authority; these fixtures are the four grant shapes the shell must render correctly.
 */
import type { Page } from "@playwright/test";
import { SPEC155_WORKSPACE } from "../../spec155/helpers/mock-api";

export type SettingsRole =
  | "member"
  | "workspace_admin"
  | "other_workspace_admin"
  | "super_admin"
  | "platform_admin";

/** Administers a workspace that is not the one on screen (EC-175-39). */
const OTHER_WORKSPACE_ID = "spec175-other-ws-002";

const PAYLOADS: Record<SettingsRole, Record<string, unknown>> = {
  member: {
    instance_admin: false,
    tenant_role: "member",
    tenant_manage: false,
    workspace_manage: false,
    administered_workspace_ids: [],
  },
  workspace_admin: {
    instance_admin: false,
    tenant_role: "member",
    tenant_manage: false,
    workspace_manage: true,
    administered_workspace_ids: [SPEC155_WORKSPACE.id],
  },
  other_workspace_admin: {
    instance_admin: false,
    tenant_role: "member",
    tenant_manage: false,
    workspace_manage: false,
    administered_workspace_ids: [OTHER_WORKSPACE_ID],
  },
  super_admin: {
    instance_admin: false,
    tenant_role: "admin",
    tenant_manage: true,
    workspace_manage: true,
    administered_workspace_ids: [SPEC155_WORKSPACE.id],
  },
  platform_admin: {
    instance_admin: true,
    tenant_role: "owner",
    tenant_manage: true,
    workspace_manage: true,
    administered_workspace_ids: [SPEC155_WORKSPACE.id],
  },
};

/** Whether a role may change the selected workspace. Shared with the model-policy fixture. */
export function workspaceManageFor(role: SettingsRole): boolean {
  return PAYLOADS[role].workspace_manage === true;
}

/** Registered after the hermetic guard, so it takes precedence (Playwright routes are LIFO). */
export async function mockSettingsAccess(page: Page, role: SettingsRole): Promise<void> {
  await page.route("**/api/v1/me/settings-access**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        user_id: `spec175-${role}`,
        workspace_id: SPEC155_WORKSPACE.id,
        ...PAYLOADS[role],
      }),
    }),
  );
}

/** `GET /tenants/{id}/workspaces/by-slug/{slug}` for the `/w/{slug}/settings` deeplink. */
export async function mockWorkspaceBySlug(page: Page): Promise<void> {
  await page.route("**/api/v1/tenants/*/workspaces/by-slug/**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(SPEC155_WORKSPACE),
    }),
  );
}

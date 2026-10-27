import { describe, expect, it } from "vitest";
import { ACCESS_CHIP_COPY, accessChip, NO_SETTINGS_ACCESS, toSettingsAccess, type SettingsAccessPayload } from "../access";

const payload = (over: Partial<SettingsAccessPayload> = {}): SettingsAccessPayload => ({
  user_id: "u1",
  instance_admin: false,
  tenant_role: "member",
  tenant_manage: false,
  workspace_id: null,
  workspace_manage: false,
  administered_workspace_ids: [],
  ...over,
});

describe("settings access (LAW-175-17 badge order)", () => {
  it("a platform admin reads as Platform admin, never Super admin", () => {
    const access = toSettingsAccess(payload({ instance_admin: true, tenant_manage: true }));
    expect(accessChip(access)).toBe("platform_admin");
    expect(ACCESS_CHIP_COPY.platform_admin.fallback).toBe("Platform admin");
  });

  it("a super admin reads as Super admin", () => {
    expect(accessChip(toSettingsAccess(payload({ tenant_manage: true })))).toBe("super_admin");
  });

  it("a workspace admin reads as Workspace admin", () => {
    expect(accessChip(toSettingsAccess(payload({ administered_workspace_ids: ["w1"] })))).toBe("workspace_admin");
  });

  it("a plain member reads as Member", () => {
    expect(accessChip(toSettingsAccess(payload()))).toBe("member");
  });

  it("platform admin implies tenant and workspace management", () => {
    const access = toSettingsAccess(payload({ instance_admin: true }));
    expect(access.tenantManage).toBe(true);
    expect(access.workspaceManage).toBe(true);
  });

  it("treats a payload with no workspace id list as a member", () => {
    const raw = {
      ...payload(),
      administered_workspace_ids: undefined,
    } as unknown as SettingsAccessPayload;
    expect(accessChip(toSettingsAccess(raw))).toBe("member");
    expect(toSettingsAccess(raw).administeredWorkspaceIds).toEqual([]);
  });

  it("fails closed before the server answers", () => {
    expect(NO_SETTINGS_ACCESS.instanceAdmin).toBe(false);
    expect(NO_SETTINGS_ACCESS.tenantManage).toBe(false);
    expect(NO_SETTINGS_ACCESS.workspaceManage).toBe(false);
    expect(accessChip(NO_SETTINGS_ACCESS)).toBe("member");
  });
});

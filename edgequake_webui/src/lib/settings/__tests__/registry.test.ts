import { describe, expect, it } from "vitest";
import { NO_SETTINGS_ACCESS, type SettingsAccess } from "../access";
import {
  canOpenSection,
  settingsCrumbLabel,
  getSection,
  scopeHref,
  searchSections,
  sectionFromPathname,
  sectionHref,
  visibleScopes,
} from "../registry";

const MEMBER: SettingsAccess = NO_SETTINGS_ACCESS;
const WS_ADMIN: SettingsAccess = { ...NO_SETTINGS_ACCESS, workspaceManage: true, administeredWorkspaceIds: ["w1"] };
const SUPER: SettingsAccess = { ...NO_SETTINGS_ACCESS, tenantManage: true, workspaceManage: true };
const PLATFORM: SettingsAccess = { ...SUPER, instanceAdmin: true };

const scopeIds = (access: SettingsAccess) => visibleScopes(access).map((s) => s.id);

describe("settings registry — visibility by role (EC-175-34, EC-175-43)", () => {
  it("a member sees only You and Workspace", () => {
    expect(scopeIds(MEMBER)).toEqual(["you", "workspace"]);
  });

  it("a workspace admin still does not see Organization or Deployment", () => {
    expect(scopeIds(WS_ADMIN)).toEqual(["you", "workspace"]);
  });

  it("a super admin sees Organization but not Deployment", () => {
    expect(scopeIds(SUPER)).toEqual(["you", "workspace", "organization"]);
  });

  it("a platform admin sees all four scopes", () => {
    expect(scopeIds(PLATFORM)).toEqual(["you", "workspace", "organization", "deployment"]);
  });

  it("the editor-only workspace sections need workspaceManage", () => {
    expect(canOpenSection(MEMBER, getSection("ingestion"))).toBe(false);
    expect(canOpenSection(MEMBER, getSection("access"))).toBe(false);
    expect(canOpenSection(WS_ADMIN, getSection("ingestion"))).toBe(true);
  });

  it("everyone can open Models (read-only for members)", () => {
    expect(canOpenSection(MEMBER, getSection("models"))).toBe(true);
  });

  it("fails closed: no grant opens nothing gated", () => {
    expect(canOpenSection(NO_SETTINGS_ACCESS, getSection("people"))).toBe(false);
    expect(canOpenSection(NO_SETTINGS_ACCESS, getSection("fleet"))).toBe(false);
  });
});

describe("settings registry — search (EC-175-33)", () => {
  it("never returns a section the caller cannot open", () => {
    const hits = searchSections(MEMBER, "pdf");
    expect(hits.map((h) => h.section.id)).not.toContain("ingestion");
    expect(hits).toHaveLength(0);
  });

  it("finds an editor section for an editor", () => {
    expect(searchSections(WS_ADMIN, "pdf").map((h) => h.section.id)).toEqual(["ingestion"]);
  });

  it("matches a keyword that is not in the label", () => {
    expect(searchSections(MEMBER, "embedding").map((h) => h.section.id)).toEqual(["models"]);
  });

  it("requires every term to match", () => {
    expect(searchSections(PLATFORM, "langfuse zebra")).toHaveLength(0);
    expect(searchSections(PLATFORM, "langfuse tracing").map((h) => h.section.id)).toEqual(["observability"]);
  });

  it("an empty query returns nothing", () => {
    expect(searchSections(PLATFORM, "   ")).toHaveLength(0);
  });
});

describe("settings registry — routes", () => {
  it("maps a bare scope to its default section", () => {
    expect(sectionFromPathname("/settings/you")).toEqual({
      section: "appearance",
      workspaceSlug: null,
      bare: true,
    });
  });

  it("maps a workspace section route", () => {
    expect(sectionFromPathname("/settings/workspace/ingestion")).toEqual({
      section: "ingestion",
      workspaceSlug: null,
    });
  });

  it("maps the workspace deeplink to Models (W-31: no redirect to /workspace)", () => {
    expect(sectionFromPathname("/w/acme/settings")).toEqual({ section: "models", workspaceSlug: "acme" });
  });

  it("maps a workspace deeplink section", () => {
    expect(sectionFromPathname("/w/acme/settings/access")).toEqual({ section: "access", workspaceSlug: "acme" });
  });

  it("ignores trailing slashes", () => {
    expect(sectionFromPathname("/settings/deployment/fleet/")?.section).toBe("fleet");
  });

  it("returns null for unknown or non-settings paths", () => {
    expect(sectionFromPathname("/settings/bogus")).toBeNull();
    expect(sectionFromPathname("/settings/workspace/nope")).toBeNull();
    expect(sectionFromPathname("/documents")).toBeNull();
  });

  it("builds hrefs that match the routes above", () => {
    expect(sectionHref("models", "acme")).toBe("/w/acme/settings");
    expect(sectionHref("ingestion", "acme")).toBe("/w/acme/settings/ingestion");
    expect(sectionHref("models")).toBe("/settings/workspace/models");
    expect(sectionHref("fleet")).toBe("/settings/deployment/fleet");
    expect(scopeHref("organization")).toBe("/settings/organization/people");
  });

  it("round-trips every section href through the matcher", () => {
    const ids = ["appearance", "graph", "browser-data", "models", "ingestion", "access", "people", "secrets", "connections", "fleet", "observability"] as const;
    for (const id of ids) {
      const href = sectionHref(id);
      expect(sectionFromPathname(href)?.section).toBe(id);
      expect(sectionFromPathname(sectionHref(id, "acme"))?.section).toBe(id);
    }
  });
});

describe("settings breadcrumbs", () => {
  const crumbs = (path: string) => {
    const segments = path.split("/").filter(Boolean);
    return segments.map((_, i) => settingsCrumbLabel(segments, i));
  };

  it("labels scope and section segments by their settings name", () => {
    expect(crumbs("/settings/you/graph")).toEqual([null, "You", "Graph display"]);
    expect(crumbs("/settings/deployment/fleet")).toEqual([null, "Deployment", "Fleet"]);
  });

  it("labels the workspace deeplink section and leaves the w/slug prefix alone", () => {
    expect(crumbs("/w/acme/settings/ingestion")).toEqual([null, null, null, "Ingestion"]);
  });

  it("does not rename segments outside settings", () => {
    expect(crumbs("/graph/abc")).toEqual([null, null]);
  });
});

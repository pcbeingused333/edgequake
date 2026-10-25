/**
 * SPEC-175 — hermetic Langfuse status for the Observability screen. The default is the common
 * first-run state: the feature is built in, but no keys are set yet.
 */
import type { Page } from "@playwright/test";

const LANGFUSE_STATUS = {
  enabled: false,
  base_url: "https://cloud.langfuse.com",
  ui_url: "https://cloud.langfuse.com",
  project_id: null,
  project_ui_url: null,
  public_key_configured: false,
  secret_key_configured: false,
  otel_feature_built: true,
  export_active: false,
  api: "auto",
  api_resolved: null,
  env_snippet: [
    "LANGFUSE_ENABLED=true",
    "LANGFUSE_BASE_URL=https://cloud.langfuse.com",
    "LANGFUSE_PUBLIC_KEY=pk-lf-...",
    "LANGFUSE_SECRET_KEY=sk-lf-...",
  ].join("\n"),
  config_requirements: [
    { name: "LANGFUSE_PUBLIC_KEY", satisfied: false },
    { name: "LANGFUSE_SECRET_KEY", satisfied: false },
  ],
};

/** `GET /api/v1/settings/langfuse`, answered with the not-configured status. */
export async function mockLangfuseStatus(page: Page): Promise<void> {
  await page.route("**/api/v1/settings/langfuse**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(LANGFUSE_STATUS),
    }),
  );
}

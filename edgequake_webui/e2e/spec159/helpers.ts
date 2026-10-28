/**
 * SPEC-159 e2e helpers — Ask handoff fixtures.
 * Screenshots → specs/159-query-action/e2e/screenshots/
 */
import { expect, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { GOTO_OPTS } from "../helpers/app-ready";
import { mockHermeticModelOffer } from "../helpers/hermetic-models";
import {
  GRAPH_FILTER_DOC_A,
  mockGraphDocumentFilterRoutes,
  seedGraphFilterTenantContext,
} from "../helpers/graph-document-filter-mocks";
import { prepareSpec155Page } from "../spec155/helpers/mock-api";
import { buildMockPdf, mockRangePdfRoute } from "../spec155/helpers/mock-pdf";

export const SHOTS_ROOT = path.resolve(
  __dirname,
  "../../../specs/159-query-action/e2e/screenshots",
);

export { GRAPH_FILTER_DOC_A };

export async function shot(page: Page, name: string): Promise<void> {
  fs.mkdirSync(SHOTS_ROOT, { recursive: true });
  await page.mouse.move(0, 0);
  // Let handoff toasts finish so they do not cover the companion card.
  await page
    .locator("[data-sonner-toast]")
    .first()
    .waitFor({ state: "detached", timeout: 4_000 })
    .catch(() => undefined);
  await page.waitForTimeout(200);
  await page.screenshot({
    path: path.join(SHOTS_ROOT, `${name}.png`),
    fullPage: false,
  });
}

export async function openNodeMenu(
  page: Page,
  nodeId?: string,
): Promise<void> {
  await page.evaluate((id) => {
    window.dispatchEvent(
      new CustomEvent("eq:e2e-open-node-menu", {
        detail: { x: 420, y: 280, nodeId: id },
      }),
    );
  }, nodeId);
  await page.getByTestId("node-context-menu").waitFor({ state: "visible" });
}

/** Neighborhood + chat stubs for Ask landing on /query. */
export async function mockAskQueryRoutes(page: Page): Promise<void> {
  // Folders API returns a bare array — `{}` from catch-alls breaks history.
  await page.route("**/api/v1/folders**", async (route) => {
    if (route.request().method() !== "GET") {
      await route.fallback();
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify([]),
    });
  });

  await page.route("**/api/v1/graph/entities/**/neighborhood**", async (route) => {
    const url = route.request().url();
    // Decode so `%3A%3A` / `::` fixtures hit the same branch.
    let decoded = url;
    try {
      decoded = decodeURIComponent(url);
    } catch {
      /* keep raw */
    }
    if (
      /MISSING|missing/i.test(url) ||
      /MISSING|missing/i.test(decoded)
    ) {
      await route.fulfill({
        status: 404,
        contentType: "application/json",
        body: JSON.stringify({ message: "Entity not found", status: 404 }),
      });
      return;
    }
    // Prefer the path segment after /entities/ for the seed node id.
    const match = url.match(/\/graph\/entities\/([^/]+)\/neighborhood/);
    let seedId = "PE8_ENTITY_A";
    if (match?.[1]) {
      try {
        seedId = decodeURIComponent(match[1]);
      } catch {
        seedId = match[1];
      }
    }
    const neighborId =
      seedId === "PE8_ENTITY_A" ? "PE8_ENTITY_B" : "PE8_ENTITY_B";
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        nodes: [
          {
            id: seedId,
            label: seedId.includes("::")
              ? seedId.split("::").pop()
              : seedId.replace(/_/g, " "),
            node_type: "CONCEPT",
            degree: { in: 0, out: 1, total: 1 },
          },
          {
            id: neighborId,
            label: "PE8 Entity B",
            node_type: "TECHNOLOGY",
            degree: { in: 1, out: 1, total: 2 },
          },
          {
            id: "PE8_EXTRA_NEIGHBOR",
            label: "PE8 Extra Neighbor",
            node_type: "CONCEPT",
            degree: { in: 1, out: 0, total: 1 },
          },
        ],
        edges: [
          {
            id: "e1",
            source: seedId,
            target: neighborId,
            relationship_type: "RELATED_TO",
            weight: 1,
          },
          {
            id: "e2",
            source: neighborId,
            target: "PE8_EXTRA_NEIGHBOR",
            relationship_type: "RELATED_TO",
            weight: 1,
          },
        ],
      }),
    });
  });

  // useConversations infinite query requires pagination.has_more (SPEC-073).
  await page.route("**/api/v1/conversations**", async (route) => {
    const method = route.request().method();
    const url = route.request().url();
    if (method === "GET" && /\/conversations\/[^/?]+/.test(url)) {
      await route.fallback();
      return;
    }
    if (method === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          items: [],
          pagination: {
            has_more: false,
            next_cursor: null,
            prev_cursor: null,
            total: 0,
          },
        }),
      });
      return;
    }
    await route.fallback();
  });

  await page.route("**/api/v1/chat/completions/**", async (route) => {
    if (route.request().method() === "POST") {
      await route.fulfill({
        status: 500,
        body: "chat must not auto-fire on Ask handoff",
      });
      return;
    }
    await route.fallback();
  });

  await page.route("**/api/v1/query/**", async (route) => {
    if (route.request().method() === "POST") {
      await route.fulfill({
        status: 500,
        body: "query must not auto-fire on Ask handoff",
      });
      return;
    }
    await route.fallback();
  });

  // Send stays disabled until the workspace query pair is eligible. The
  // hermetic page has no backend, so the composer must see a matching offer.
  await page.route("**/api/v1/config/effective**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        workspace_id: null,
        policy_mode: "unconstrained",
        usages: [
          {
            usage: "query",
            provider: "ollama",
            provider_display_name: "Ollama",
            model: "gemma4:latest",
            connection_id: null,
            reasoning_effort: null,
            source: "compiled_default",
            locked: false,
            eligible: true,
            broken_reason: "none",
          },
        ],
      }),
    }),
  );
  await mockHermeticModelOffer(page);
}

export async function prepareGraphAskPage(page: Page): Promise<void> {
  await page.setViewportSize({ width: 1440, height: 900 });
  await mockGraphDocumentFilterRoutes(page);
  await mockAskQueryRoutes(page);
  await seedGraphFilterTenantContext(page);
  await page.goto(
    `/graph?document=${GRAPH_FILTER_DOC_A}&stream=0`,
    GOTO_OPTS,
  );
  await expect(page.getByText(/2 nodes · 1 edge/i)).toBeVisible({
    timeout: 20_000,
  });
}

const DOC = {
  id: GRAPH_FILTER_DOC_A,
  pdf_id: GRAPH_FILTER_DOC_A,
  title: "manifold_2605.13438v3.pdf",
  file_name: "manifold_2605.13438v3.pdf",
  status: "completed",
  source_type: "pdf",
  mime_type: "application/pdf",
  page_count: 12,
  content:
    "<!-- edgequake-page:1 -->\n# Page one\n\n<!-- edgequake-page:2 -->\n# Page two\n",
  created_at: "2026-10-01T06:00:00Z",
  updated_at: "2026-10-01T06:30:00Z",
};

export async function prepareDocumentAskPage(page: Page): Promise<void> {
  await prepareSpec155Page(page, { documents: [DOC] });
  await mockAskQueryRoutes(page);
  await page.route(/\/api\/v1\/documents\/pdf\/[^/?]+$/, (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        pdf_id: GRAPH_FILTER_DOC_A,
        document_id: GRAPH_FILTER_DOC_A,
        filename: DOC.file_name,
        file_size_bytes: 1,
        content_type: "application/pdf",
        markdown_content: DOC.content,
        is_processed: true,
      }),
    }),
  );
  const pdf = buildMockPdf(12);
  await mockRangePdfRoute(page, pdf);
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`/documents/${GRAPH_FILTER_DOC_A}?page=2`, GOTO_OPTS);
  await expect(page.getByTestId("detail-ask-about-page")).toBeVisible({
    timeout: 30_000,
  });
}

export async function prepareMarkdownAskPage(page: Page): Promise<void> {
  const md = {
    id: GRAPH_FILTER_DOC_A,
    title: "notes.md",
    file_name: "notes.md",
    status: "completed",
    source_type: "markdown",
    mime_type: "text/markdown",
    content: "# Notes\n\nNo page markers here.",
    created_at: "2026-10-01T06:00:00Z",
    updated_at: "2026-10-01T06:30:00Z",
  };
  await prepareSpec155Page(page, { documents: [md] });
  await mockAskQueryRoutes(page);
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`/documents/${GRAPH_FILTER_DOC_A}`, GOTO_OPTS);
  await expect(page.getByTestId("detail-ask-about-page")).toBeVisible({
    timeout: 30_000,
  });
}

export async function mockDocumentLineage(page: Page): Promise<void> {
  await page.route(/\/api\/v1\/documents\/[^/]+\/lineage/, async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        lineage: {
          document_name: "manifold_2605.13438v3.pdf",
          chunks: [
            {
              chunk_id: "chunk-ask-1",
              chunk_index: 0,
              page_start: 2,
              page_end: 2,
              start_line: 10,
              end_line: 18,
              entity_ids: ["PE8_ENTITY_A"],
            },
          ],
          entities: {
            PE8_ENTITY_A: {
              entity_id: "PE8_ENTITY_A",
              entity_name: "PE8 Entity A",
              extraction_count: 1,
              sources: [{ chunk_ids: ["chunk-ask-1"] }],
            },
          },
        },
      }),
    });
  });
}

export const PRIOR_CONV_ID = "conv-spec159-prior-001";

/** Seed an active prior chat + mock its list/detail for new-conversation asserts. */
export async function seedPriorConversation(page: Page): Promise<void> {
  const now = new Date().toISOString();
  const detail = {
    id: PRIOR_CONV_ID,
    title: "Prior chat about widgets",
    created_at: now,
    updated_at: now,
    mode: "mix",
    message_count: 2,
    messages: [
      {
        id: "msg-user-prior",
        role: "user",
        content: "PRIOR_CHAT_MARKER unique question",
        created_at: now,
      },
      {
        id: "msg-asst-prior",
        role: "assistant",
        content: "PRIOR_CHAT_MARKER unique answer",
        created_at: now,
      },
    ],
  };

  await page.route("**/api/v1/conversations**", async (route) => {
    const url = route.request().url();
    const method = route.request().method();
    if (method === "GET" && url.includes(`/conversations/${PRIOR_CONV_ID}`)) {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(detail),
      });
      return;
    }
    if (method === "GET") {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          items: [
            {
              id: PRIOR_CONV_ID,
              title: detail.title,
              updated_at: now,
              created_at: now,
              mode: "mix",
              message_count: 2,
            },
          ],
          pagination: {
            has_more: false,
            next_cursor: null,
            prev_cursor: null,
            total: 1,
          },
        }),
      });
      return;
    }
    await route.fallback();
  });

  await page.evaluate((conversationId) => {
    localStorage.setItem(
      "edgequake-query-ui",
      JSON.stringify({
        state: {
          historyPanelOpen: true,
          activeConversationId: conversationId,
          filters: {
            mode: null,
            archived: false,
            pinned: null,
            folderId: null,
            search: "",
            dateFrom: null,
            dateTo: null,
          },
          sort: { field: "updated_at", order: "desc" },
        },
        version: 0,
      }),
    );
  }, PRIOR_CONV_ID);
}

export async function prepareQueryPage(page: Page): Promise<void> {
  await page.setViewportSize({ width: 1440, height: 900 });
  await prepareSpec155Page(page);
  await mockAskQueryRoutes(page);
  await page.goto("/query", GOTO_OPTS);
  await expect(page.getByTestId("query-split-toggle")).toBeVisible({
    timeout: 30_000,
  });
}

export function composerInput(page: Page) {
  return page.locator("textarea.query-input").first();
}

export async function expectPriorChatInHistory(page: Page): Promise<void> {
  const closePane = page.getByTestId("companion-close");
  if (await closePane.isVisible().catch(() => false)) {
    await closePane.click();
  }
  const title = page.getByText("Prior chat about widgets");
  if (await title.isVisible().catch(() => false)) {
    await expect(title).toBeVisible();
    return;
  }
  const toggle = page.getByTestId("query-history-toggle");
  await expect(toggle).toBeVisible({ timeout: 10_000 });
  if ((await toggle.getAttribute("aria-pressed")) !== "true") {
    await toggle.click();
  }
  await expect(title).toBeVisible({ timeout: 10_000 });
}

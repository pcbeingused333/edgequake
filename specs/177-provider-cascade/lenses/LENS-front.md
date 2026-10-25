# Lens — Front

Parent: [README](../README.md) · Architecture: [10-frontend-architecture](../10-frontend-architecture.md)

## What this lens owns

[LAW-177-6](../01-first-principles.md) and [LAW-177-7](../01-first-principles.md).
`ProviderOfferEditor` takes `parent`. It does not fork into three pages
that each know a different rule. The payload is the list. The browser
does not import `models.toml` and does not recompute the parent set.
[EC-177-41](../11-edge-cases.md).

## Files

| File | Job |
|------|-----|
| `src/lib/settings/provider-offer.ts` | Pure draft helpers. No React. |
| `src/hooks/use-provider-offer.ts` | Query keys and invalidation. |
| `src/components/settings/providers/provider-offer-editor.tsx` | The one editor. |
| `src/lib/settings/registry.ts` | `providers` and `org-providers`. |
| `src/components/settings/sections/index.tsx` | The only section map. |
| `src/components/settings/sections/people-section.tsx` | Loses `TenantModelCeilingCard`. |
| `src/components/settings/connections-card.tsx` | Withdraw, badge, no tenant header on create. |

`TenantModelCeilingCard` stays the model table. The editor passes one
provider's rows into it. Workspace narrowing does the same. The role
editor keeps reading `GET /models/eligible` and does not filter again.

## Draft rules

- Dirty when slugs, `offer_mode`, or model rows differ from the GET.
- A provider switched off drops its model rows from the PUT.
  [EC-177-35](../11-edge-cases.md).
- `useServerBackedDraft` remains the draft. No Zustand key for the offer.
- Create connection opts out of `X-Tenant-ID`. [EC-177-47](../11-edge-cases.md).

## Query keys

`deployment-providers`, `tenant-provider-offer`, `workspace-provider-offer`.
A withdraw invalidates the deployment key and the tenant keys. The chip's
eligible query is invalidated with the workspace offer so the menu cannot
show a slug the save just removed.

## Tests

`provider-offer.test.ts` for the pure helpers. `registry.test.ts` for
search. Playwright under `e2e/spec177` on `mock-api` only.

## Out of this lens

The SQL transaction and the lock bypass inside `resolve_usage`.

# Lens — Front

Parent: [README](../README.md) · Module: [10-frontend-architecture](../10-frontend-architecture.md)

## What this lens owns

The client half of [LAW-174-5](../01-first-principles.md): one module,
two route trees, no second policy.

## Module

`edgequake_webui/src/lib/clearance/` holds `types.ts`, `api.ts`, and
`use-clearance.ts`. Components under `src/components/documents/` receive
chips as data. They do not import rule matching.

Dashboard routes and `app/w/[slug]/` routes share the React Query keys
`['clearance', workspaceId]` and the document list key. Relabel, enable,
and role save invalidate clearance, documents, graph, and stats together.
A later 404 drops that document id from the cache.

## Preview

`api.ts` posts to simulate and the Access card renders the titles. No
header, no `?asRole=`, no second document query key
([RB-174-05](../17-roadblocks.md)). A non-steward gets 403
([EC-174-07](../12-edge-cases.md)).

## Tests this lens adds

Playwright file `edgequake_webui/e2e/document-clearance.spec.ts` from
[14-e2e-test-matrix](../14-e2e-test-matrix.md): hidden title absent on
list, detail, and graph; enable dialog with no checkbox; empty state;
one Access row on upload.

## Reject in review

- Filtering graph nodes in the canvas.
- Duplicating fetch calls inside a page component.
- Storing the compiled subject in localStorage as an authorization cache.

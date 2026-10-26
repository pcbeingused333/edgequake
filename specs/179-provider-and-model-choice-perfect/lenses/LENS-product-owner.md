# Lens — Product owner

Parent: [15-cross-ref](../15-cross-ref.md)

## WHY this lens

The filter is a promise to the org admin: "if I turn it off, it is off."
SPEC-178 made that promise on the query page. Ingest still breaks it. The
product work is to make the promise true, and to make the screen admit when
a change will break existing work.

## Decisions this lens owns

- Fail closed, including documents already queued. A banner and a failed
  task beat a silent model swap. [LAW-179-2](../01-first-principles.md).
- The admin confirms a destructive write. The product does not pick a
  replacement model. [LAW-179-8](../01-first-principles.md).
- Discovered models stay off until registered. A refresh is not a grant.
- Inherit stays. Empty means the parent layer. It does not mean "whatever
  the server process was started with", unless an operator lock says so.

## Acceptance the owner will demo

1. Withdraw Gemini. Upload a PDF. The task fails and names Workspace → Models.
2. Add a private OpenAI-compatible endpoint, test it, refresh, register one
   chat model and one embedder, offer it to one org.
3. A member's chip does not list the model the org ceiling removed.

## Out of scope

Vault, AWS Secrets Manager, a marketplace of providers, per-user model
budgets, automatic failover to the next model.

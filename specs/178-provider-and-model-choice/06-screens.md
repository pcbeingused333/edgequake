# 06 — Screens

Parent: [README](README.md) · Prev: [05-choice-law](05-choice-law.md) ·
Next: [07-data-and-db-contract](07-data-and-db-contract.md)

ASCII is the wire. Copy and focus order are in
[09-ux-ui-spec](09-ux-ui-spec.md). Components are in
[10-frontend-architecture](10-frontend-architecture.md).

## Query chip, workspace scope, usage query

```text
  Composer
  +------------------------------------------------------------------+
  |  Ask about this workspace                                        |
  |                                                                  |
  |  [ Inherit · Gemma 3 · This workspace v ]   [ Stream ]  [ Send ] |
  +------------------------------------------------------------------+
                              |
                              v
  +-------------------------------------------+
  | Search models                             |
  +-------------------------------------------+
  | Inherit                                   |
  |   Gemma 3                                 |
  |   This workspace                          |
  +-------------------------------------------+
  | OpenAI                                    |
  |   GPT-5 nano                         [x]  |
  +-------------------------------------------+
  |   (no second provider: it is not offered) |
  +-------------------------------------------+
```

The open list is `eligible(workspace, query)` plus Inherit.
[S1](03-product-spec.md) · [EC-178-01](11-edge-cases.md).

Inherit's second line is `source` from `GET /config/effective`, mapped
in [09-ux-ui-spec](09-ux-ui-spec.md). [EC-178-06](11-edge-cases.md).

## Chip when the usage is locked

```text
  +------------------------------------------------------------------+
  |  [ Locked · extract pin · operator ]          [ Stream ] [ Send ]|
  +------------------------------------------------------------------+
```

The control does not open. A request that still sends a pair receives
`422 usage_locked`. [EC-178-10](11-edge-cases.md) · [EC-178-37](11-edge-cases.md).

## Chip when the eligible set is empty

```text
  +------------------------------------------------------------------+
  |  [ Inherit · Broken · no model for answers ]     [ Send disabled ]|
  +------------------------------------------------------------------+
  |  An admin needs to allow a model for answers in this workspace.  |
  +------------------------------------------------------------------+
```

Send stays disabled while Inherit is broken. The member is not offered
a hidden inventory model. [EC-178-18](11-edge-cases.md).

## Workspace Models, one usage row

```text
  Models
  +------------------------------------------------------------------+
  | Providers this workspace keeps          [ Offer editor ]         |
  +------------------------------------------------------------------+
  | Answers                                                         |
  |   [ GPT-5 nano            v ]                                    |
  |   Effective: GPT-5 nano · This workspace                         |
  +------------------------------------------------------------------+
  | Extract                                                         |
  |   [ Inherit               v ]                                    |
  |   Effective: Gemma 3 · Organization                              |
  +------------------------------------------------------------------+
  | Embeddings                                                      |
  |   [ text-embedding-3-small v ]   dimension 1536                  |
  |   Rebuild embeddings …                                           |
  +------------------------------------------------------------------+
  | Vision                                                          |
  |   [ Broken · llava ]                                             |
  |   This model is no longer allowed. Pick another, or Inherit.     |
  +------------------------------------------------------------------+
```

Each closed list is that usage's eligible set. The broken vision row
shows the stored id and is not inside the list. Saving Inherit clears
the role. [S4](03-product-spec.md) · [S5](03-product-spec.md) ·
[EC-178-23](11-edge-cases.md).

The embeddings row shows the card dimension. Rebuild is a separate
control. [EC-178-14](11-edge-cases.md) · [EC-178-38](11-edge-cases.md).

## Chat family control

Used on create-workspace, onboarding, and anywhere the form writes
`llm_model` rather than one role.

```text
  Default chat model
  +------------------------------------------------------------------+
  | [ GPT-5 nano v ]                                                 |
  | Used for extract, answers, summary, and keywords                |
  | unless a row below overrides one of them.                       |
  +------------------------------------------------------------------+
```

The list is the intersection of the four chat usages.
[EC-178-29](11-edge-cases.md). Per-usage rows on the same page may show
a wider list for that one usage.

## Create workspace, tenant scope

```text
  New workspace
  +------------------------------------------------------------------+
  | Name [ Finance ]                                                 |
  |                                                                  |
  | Default chat model   [ Inherit organization default v ]         |
  | Embedding model      [ text-embedding-3-small v ]  dim 1536     |
  |                                                                  |
  | Models follow the organization. This workspace can narrow them  |
  | after it exists.                                                 |
  +------------------------------------------------------------------+
```

No workspace id is sent to the eligible route. The query is
`tenant_id` + `usage`. [EC-178-15](11-edge-cases.md).

## Deployment defaults

```text
  Fleet defaults
  +------------------------------------------------------------------+
  | Chat      [ Gemma 3 v ]                                          |
  | Embedding [ nomic-embed-text v ]   dim 768                       |
  | Vision    [ Inherit v ]                                          |
  |                                                                  |
  | These apply when a workspace and its organization leave the     |
  | choice empty. They do not override a workspace that has chosen. |
  +------------------------------------------------------------------+
```

The lists are deployment scope. A workspace narrowing does not remove
a row here. [EC-178-16](11-edge-cases.md).

## Composer after the admin changes the ceiling

```text
  before                              after the ceiling save
  +---------------------------+       +---------------------------+
  | [ GPT-5 nano v ]          |       | [ Inherit · Gemma 3 v ]   |
  +---------------------------+       +---------------------------+
                                      | GPT-5 nano left the list  |
                                      +---------------------------+
```

The pair was stored for this workspace. The eligible refetch drops it.
The chip returns to Inherit and clears that workspace's slot.
[EC-178-04](11-edge-cases.md) · [EC-178-28](11-edge-cases.md).

## Two workspaces

```text
  workspace Finance                         workspace Legal
  +----------------------------+            +----------------------------+
  | slot finance: gpt-5-nano   |            | slot legal: gemma3         |
  +----------------------------+            +----------------------------+
         ^                                          ^
         |                                          |
         +------------ one browser store ----------+
                      keyed by workspace id
```

[EC-178-27](11-edge-cases.md).

## What none of these screens show

```text
  x  a provider switch the parent offer already removed
  x  a disabled checkbox the user can turn on
  x  mock
  x  a discovery-only id with no card
  x  the process model as a silent extra row
  x  an API key
```

[LAW-178-1](01-first-principles.md) · [EC-178-20](11-edge-cases.md) ·
[EC-178-22](11-edge-cases.md).

# 10 — Screens

Parent: [README](README.md) · Prev: [09-ux-ui-spec](09-ux-ui-spec.md) ·
Next: [11-frontend-architecture](11-frontend-architecture.md)

## WHY

Wireframes fix the order of information before components get rearranged.
They are ASCII so they diff. The implementation matches this order, not a
new one invented in the component.

## Deployment · Connections

```text
+------------------------------------------------------------------+
| Connections                                         [Add provider]|
+------------------------------------------------------------------+
| openai-prod                                          cloud       |
| https://api.openai.com/v1                                       |
| Key ending a91f          [Rotate]  [Test]  [Delete]             |
| Test passed · 142 ms · 3 minutes ago                            |
+------------------------------------------------------------------+
| Add provider                                                    |
| Shape [OpenAI compatible v]   Slug [________]                   |
| Base URL [________________________]                            |
| Key      [____________________]  "stored encrypted, never shown"|
|                                     [Test]  [Save]              |
+------------------------------------------------------------------+
```

## Deployment · Providers

```text
+------------------------------------------------------------------+
| openai-prod     Test passed · 142 ms     Refreshed 3m ago       |
| [Refresh]  [Withdraw]                                           |
| +3 new   -1 gone                                                |
| Search [________]   [Chat v]   [Select visible]                 |
| [x] gpt-5.6-luna     chat · vision · 400k                       |
| [ ] embed-small      embed · dim [1536]                         |
|                              [Register selected]                |
+------------------------------------------------------------------+
| gemini (env)    Key from the server     [Refresh] [Withdraw]    |
+------------------------------------------------------------------+
```

## Organization · Providers

```text
+------------------------------------------------------------------+
| Offered by the deployment. Uncheck to hide from this org.       |
| [x] openai-prod                                                 |
| [x] gemini                                                      |
| [ ] ollama                                                      |
| Models                                                          |
| openai-prod / gpt-5.6-luna   [extract] [query] [vision] ...     |
| Unsaved changes                              [Discard] [Save]   |
+------------------------------------------------------------------+
```

Save that removes a used model opens:

```text
+----------------------------------------------+
| This change breaks 2 assignments            |
|  Finance / extract  openai-prod gpt-5.6-luna |
|  Finance / query    openai-prod gpt-5.6-luna |
| They will fail until an admin picks another.|
|                        [Cancel] [Save anyway]|
+----------------------------------------------+
```

## Workspace · Models

Same matrix, parent is the org offer. Under it, the role editor:

```text
  Extract    [Inherit · gemini / gemini-2.5-flash v]
  Query      [openai-prod / gpt-5.6-luna          v]
  Embedding  [openai-prod / embed-small · 1536    v]
  Vision     [Inherit · openai-prod / gpt-5.6-luna v]
```

A broken role shows the banner from [09-ux-ui-spec](09-ux-ui-spec.md) inside
the row, not a toast.

## Query chip

```text
  [Inherit · gemini-2.5-flash v]
     Inherit · gemini / gemini-2.5-flash
     openai-prod
       gpt-5.6-luna     chat · vision
     gemini
       gemini-2.5-flash chat
```

## Upload

The dropzone gains one control, "Vision model", using the same menu with
`usage=vlm`. Parser and reasoning effort stay. There is no second card.

## 390px

The provider row stacks: name, status, actions, then models as blocks.
Screenshots: `e2e/screenshots/round-NN/` at 1440 and 390, light and dark.

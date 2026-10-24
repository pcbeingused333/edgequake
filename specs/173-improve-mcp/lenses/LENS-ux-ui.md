# Lens — UX and UI

Parent: [README](../README.md) · Words the agent reads: [05](../05-contract-delta.md) · Law: [LAW-173-3](../01-first-principles.md)

The user of this surface is the person watching the agent, and the agent itself. There is no new screen. The UI is the tool text, the error code, and the document status the product already shows.

## What the person should see

| Moment | What is true |
|--------|----------------|
| Agent starts an upload | The tool result tells the host to PUT and tells the model not to paste the URL or the file ([LAW-173-3](../01-first-principles.md)) |
| File is too large | `eq/budget_exceeded`, and no URL to click ([EC-173-07](../06-edge-cases.md)) |
| Host has not PUT yet | Commit says the bytes are missing (`eq/not_found`), which the agent can retry after the PUT ([EC-173-11](../06-edge-cases.md)) |
| URL is old | `eq/not_found` on commit, 404 on the relay. The agent starts a new begin. There is no "refresh link" control ([EC-173-03](../06-edge-cases.md)) |
| Pipeline is running | Status stays `pending` until `eq_task_get` reports `indexed`. The agent must not tell the user the PDF is searchable before that |

## What the person should not see

- The presigned query string in a log pane or a copied debug bundle ([EC-173-14](../06-edge-cases.md))
- A browser download of the original that skips the login the Web UI already requires ([LAW-173-3](../01-first-principles.md))
- Two different error dialects. Codes stay the `eq/*` set in [05](../05-contract-delta.md)

## Copy

The instruction in [05](../05-contract-delta.md) is the whole new sentence. It names the order (begin, PUT, commit, poll) and the prohibition (do not repeat the URL, do not paste the file). Tool descriptions stay short enough that `tools/list` does not become a manual.

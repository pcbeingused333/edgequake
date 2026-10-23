# Lens — Product owner

Parent: [README](../README.md) · Laws: [01](../01-first-principles.md) · Incidents: [02](../02-incident-catalogue.md)

## What the user sees

A CV sat on "processing" for two hours and then became "failed". Re-uploading the same file, alone, worked. The failure text said the task timed out after 7200 seconds and `retry_count` was 1. Nothing in that sentence told the operator the work had finished in 26 seconds, or that a checkpoint was sitting unused.

[WHY-171-2](../00-why.md) and [WHY-171-4](../00-why.md). [LAW-171-4](../01-first-principles.md).

## What changes for them

| Today | After |
|-------|--------|
| Processing for up to two hours with no new log | The phase is visible. A stuck persist ends in about 15 minutes, sooner if the lock wait hits 30 s. |
| Failed, and the checkpoint is thrown away | Failed only when the retry budget is spent. Until then the document stays indexing and the next attempt skips extraction. |
| "retry_count: 1" reads as a retry | The text says `attempt N of M` and names the stage. |
| A deploy can leave the document processing because workers lose the pool | Workers stop before the pool closes. The row is cancelled or reclaimable. |

## What does not change

The user does not get a longer timeout. A real merge error (`GraphMerge`) is still a hard failure. A deleted document is still a hard failure. The UI does not grow a new control in this spec. Status values stay `indexing`, `failed`, `completed`.

## Acceptance the owner can watch

- Incident A’s shape, reproduced with a held row lock, leaves processing before 15 minutes and says the stage ([T-171-28](../08-e2e-test-matrix.md)).
- Releasing the lock before the retry makes the same document completed without a second extraction bill ([T-171-29](../08-e2e-test-matrix.md)).
- The message a human reads contains the stage and `attempt N of M` ([T-171-10](../08-e2e-test-matrix.md)).

[LAW-171-2](../01-first-principles.md) is the promise: a stall is a retry, a bad document is not.

"use client";

import { useCallback, useState } from "react";

/**
 * A local draft that restarts from the server baseline whenever the baseline changes
 * (save, refetch, workspace switch). Derived during render, so no frame shows the old draft
 * against the new baseline. Pass a stable `saved` reference (query data, store field).
 */
export function useServerBackedDraft<S, D>(saved: S, toDraft: (saved: S) => D) {
  const [form, setForm] = useState<{ base: S; draft: D }>(() => ({ base: saved, draft: toDraft(saved) }));
  if (form.base !== saved) setForm({ base: saved, draft: toDraft(saved) });

  const draft = form.base === saved ? form.draft : toDraft(saved);

  const setDraft = useCallback(
    (update: (current: D) => D) => setForm((current) => ({ base: current.base, draft: update(current.draft) })),
    [],
  );
  const reset = useCallback(() => setForm({ base: saved, draft: toDraft(saved) }), [saved, toDraft]);

  return { draft, setDraft, reset };
}

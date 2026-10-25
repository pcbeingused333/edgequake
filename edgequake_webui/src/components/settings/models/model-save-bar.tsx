"use client";

import { Button } from "@/components/ui/button";
import { toast } from "sonner";

/** Run a save, toast the outcome, and report whether it succeeded. Shared by every model editor. */
export async function saveWithToast(action: () => Promise<unknown>, success: string): Promise<boolean> {
  try {
    await action();
    toast.success(success);
    return true;
  } catch (err) {
    toast.error(err instanceof Error ? err.message : "Save failed");
    return false;
  }
}

interface ModelSaveBarProps {
  dirty: boolean;
  saving: boolean;
  onSave: () => void;
  onDiscard: () => void;
}

/** Discard and Save for one editor. Both stay disabled until the draft differs from the server. */
export function ModelSaveBar({ dirty, saving, onSave, onDiscard }: ModelSaveBarProps) {
  return (
    <div className="flex items-center justify-end gap-2">
      <Button type="button" variant="ghost" disabled={!dirty || saving} onClick={onDiscard}>
        Discard
      </Button>
      <Button type="button" disabled={!dirty || saving} onClick={onSave}>
        {saving ? "Saving…" : "Save"}
      </Button>
    </div>
  );
}

"use client";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
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

/**
 * Discard and Save for one editor. Both stay disabled until the draft differs from the server.
 * While there are unsaved changes the bar sticks to the bottom of the scroll area and says so.
 */
export function ModelSaveBar({ dirty, saving, onSave, onDiscard }: ModelSaveBarProps) {
  return (
    <div
      data-testid="model-save-bar"
      data-dirty={dirty ? "true" : "false"}
      className={cn(
        "flex flex-wrap items-center justify-end gap-2",
        dirty && "sticky bottom-0 z-10 rounded-md border bg-background/95 px-3 py-2 shadow-sm backdrop-blur",
      )}
    >
      {dirty ? (
        <span role="status" className="mr-auto text-sm font-medium text-amber-700 dark:text-amber-400">
          Unsaved changes
        </span>
      ) : null}
      <Button type="button" variant="ghost" disabled={!dirty || saving} onClick={onDiscard}>
        Discard
      </Button>
      <Button type="button" disabled={!dirty || saving} onClick={onSave}>
        {saving ? "Saving…" : "Save"}
      </Button>
    </div>
  );
}

"use client";

import { LabelPopover, NameForm } from "@/components/clearance/label-popover";
import { Button } from "@/components/ui/button";
import { clearanceApi } from "@/lib/clearance/api";
import { reportError } from "@/lib/clearance/errors";
import { invalidateClearanceViews } from "@/lib/clearance/use-clearance";
import { compartmentTags } from "@/lib/clearance/summary";
import type { ClearanceTag } from "@/lib/clearance/types";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { X } from "lucide-react";
import type { ReactNode } from "react";

/** Add and remove departments and projects. Removing one still in use is refused by the server. */
export function VocabularyEditor({
  workspaceId,
  tags,
  trigger,
}: {
  workspaceId: string;
  tags: ClearanceTag[];
  trigger: ReactNode;
}) {
  const queryClient = useQueryClient();
  const refresh = () => invalidateClearanceViews(queryClient, workspaceId);
  const vocabulary = compartmentTags(tags);

  const add = useMutation({
    mutationFn: (value: string) => clearanceApi.createTag(workspaceId, value),
    onSuccess: refresh,
    onError: (error) => reportError(error, "Could not add that name."),
  });
  const remove = useMutation({
    mutationFn: (tag: ClearanceTag) => clearanceApi.deleteTag(workspaceId, tag.id),
    onSuccess: refresh,
    onError: (error) => reportError(error, "Could not remove that name."),
  });

  return (
    <LabelPopover trigger={trigger} title="Departments and projects" testId="vocabulary-editor">
      {vocabulary.length === 0 ? (
        <p className="text-sm text-muted-foreground">Nothing yet. Add the first one below.</p>
      ) : (
        <ul className="max-h-56 space-y-0.5 overflow-y-auto -mx-2">
          {vocabulary.map((tag) => (
            <li key={tag.id} className="flex items-center justify-between gap-2 rounded-md px-2 py-1 hover:bg-muted">
              <span className="min-w-0 truncate text-sm">{tag.value}</span>
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7 shrink-0 text-muted-foreground"
                aria-label={`Remove ${tag.value}`}
                disabled={remove.isPending}
                onClick={() => remove.mutate(tag)}
              >
                <X className="size-3.5" aria-hidden />
              </Button>
            </li>
          ))}
        </ul>
      )}
      <NameForm
        placeholder="Department or project"
        actionLabel="Add"
        onSubmit={(value) => add.mutateAsync(value)}
      />
    </LabelPopover>
  );
}

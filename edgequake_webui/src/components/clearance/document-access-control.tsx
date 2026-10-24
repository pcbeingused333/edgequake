"use client";

import {
  ChecklistGroup,
  ChoiceField,
  LabelPopover,
  type SelectChoice,
} from "@/components/clearance/label-popover";
import { Button } from "@/components/ui/button";
import { clearanceApi } from "@/lib/clearance/api";
import { reportError } from "@/lib/clearance/errors";
import { CEILINGS } from "@/lib/clearance/rules";
import { ceilingLabel, documentLabelSummary, tagOptions } from "@/lib/clearance/summary";
import type { ClassificationWord } from "@/lib/clearance/types";
import { invalidateClearanceViews, useClearanceTags, useClearanceSettings } from "@/lib/clearance/use-clearance";
import { useTenantStore } from "@/stores/use-tenant-store";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

export interface AccessibleDocument {
  id: string;
  classification?: string | null;
  compartments?: Array<{ id: string; value: string }> | null;
}

interface LabelDraft {
  classification: ClassificationWord;
  compartmentIds: string[];
}

function draftOf(document: AccessibleDocument): LabelDraft {
  return {
    classification: (document.classification ?? "unverified") as ClassificationWord,
    compartmentIds: (document.compartments ?? []).map((item) => item.id),
  };
}

function sameLabel(a: LabelDraft, b: LabelDraft): boolean {
  return (
    a.classification === b.classification &&
    a.compartmentIds.length === b.compartmentIds.length &&
    a.compartmentIds.every((id) => b.compartmentIds.includes(id))
  );
}

/** The Access value in document details. Owners and admins can change it. Others read it. */
export function DocumentAccessControl({ document }: { document: AccessibleDocument }) {
  const workspaceId = useTenantStore((state) => state.selectedWorkspaceId);
  const settings = useClearanceSettings(workspaceId);
  const canManage = settings.data?.can_manage === true;
  const steward = settings.data?.steward === true;
  const tags = useClearanceTags(workspaceId, canManage);
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState<LabelDraft>(() => draftOf(document));

  const summary = documentLabelSummary(
    document.classification ?? "unverified",
    (document.compartments ?? []).map((item) => item.value),
    steward,
  );

  const relabel = useMutation({
    mutationFn: (next: LabelDraft) =>
      clearanceApi.relabel(document.id, next.classification, next.compartmentIds),
    onSuccess: () => invalidateClearanceViews(queryClient, workspaceId ?? ""),
    onError: (error) => reportError(error, "Could not change this document's access."),
  });

  if (!canManage || !workspaceId) {
    return (
      <span className="min-w-0 truncate text-sm" data-testid="access-property">
        {summary}
      </span>
    );
  }

  const saved = draftOf(document);
  const choices: SelectChoice<ClassificationWord>[] = [
    ...CEILINGS.map((value) => ({ value, label: ceilingLabel(value) })),
    // Unverified is shown only while it is the current value. Choosing a ceiling clears it.
    ...(saved.classification === "unverified"
      ? [{ value: "unverified" as ClassificationWord, label: "Not reviewed" }]
      : []),
  ];

  async function handleOpenChange(next: boolean) {
    if (next) {
      setDraft(saved);
      setOpen(true);
      return;
    }
    if (sameLabel(draft, saved)) {
      setOpen(false);
      return;
    }
    try {
      await relabel.mutateAsync(draft);
      setOpen(false);
    } catch {
      // Reported by the mutation; keep the popover open so the choice can be retried.
    }
  }

  return (
    <LabelPopover
      title="Document access"
      open={open}
      onOpenChange={(next) => void handleOpenChange(next)}
      align="end"
      testId="document-access-editor"
      trigger={
        <Button
          variant="outline"
          size="sm"
          className="h-7 max-w-[16rem] justify-start truncate font-normal"
          data-testid="access-property"
        >
          {summary}
        </Button>
      }
    >
      <ChoiceField<ClassificationWord>
        label="Classification"
        value={draft.classification}
        choices={choices}
        onChange={(classification) => setDraft((current) => ({ ...current, classification }))}
      />
      <ChecklistGroup
        label="Departments and projects"
        testId="document-access-departments"
        options={tagOptions(tags.data ?? [])}
        selected={draft.compartmentIds}
        onToggle={(id) =>
          setDraft((current) => ({
            ...current,
            compartmentIds: current.compartmentIds.includes(id)
              ? current.compartmentIds.filter((item) => item !== id)
              : [...current.compartmentIds, id],
          }))
        }
        emptyText="No departments yet. Add them in Settings, under Access."
      />
    </LabelPopover>
  );
}

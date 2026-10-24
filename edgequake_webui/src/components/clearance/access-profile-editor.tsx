"use client";

import { ChecklistGroup, ChoiceField, LabelPopover } from "@/components/clearance/label-popover";
import { Button } from "@/components/ui/button";
import { clearanceApi } from "@/lib/clearance/api";
import { reportError } from "@/lib/clearance/errors";
import {
  CEILINGS,
  draftFromRules,
  rulesFromDraft,
  sameDraft,
  type Ceiling,
  type ProfileDraft,
} from "@/lib/clearance/rules";
import { invalidateClearanceViews } from "@/lib/clearance/use-clearance";
import { ceilingLabel, tagOptions } from "@/lib/clearance/summary";
import type { ClearanceTag, SecurityRoleView } from "@/lib/clearance/types";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState, type ReactNode } from "react";

const CEILING_CHOICES = CEILINGS.map((value) => ({ value, label: ceilingLabel(value) }));

function toggled(ids: string[], id: string): string[] {
  return ids.includes(id) ? ids.filter((item) => item !== id) : [...ids, id];
}

/**
 * Edit one access profile. The draft starts from the saved rules each time the
 * popover opens, and saves when it closes. Preview saves first, so it shows what the
 * user just chose.
 */
export function AccessProfileEditor({
  workspaceId,
  role,
  tags,
  trigger,
  onPreview,
}: {
  workspaceId: string;
  role: SecurityRoleView;
  tags: ClearanceTag[];
  trigger: ReactNode;
  onPreview: (role: SecurityRoleView) => void;
}) {
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const saved = useMemo(() => draftFromRules(role.rules), [role.rules]);
  const [draft, setDraft] = useState<ProfileDraft>(saved);
  // A profile with no rules has nothing saved yet, so closing it always saves the draft.
  const dirty = role.rules.length === 0 || !sameDraft(draft, saved);
  const departments = tagOptions(tags);
  // Only a department the profile already grants can be excepted from it.
  const exceptOptions = departments.filter((option) => draft.departmentIds.includes(option.id));

  const save = useMutation({
    mutationFn: (draftToSave: ProfileDraft) =>
      clearanceApi.saveRules(workspaceId, role.id, rulesFromDraft(draftToSave)),
    onSuccess: () => invalidateClearanceViews(queryClient, workspaceId),
    onError: (error) => reportError(error, "Could not save this profile."),
  });

  /** Save if anything changed. Returns false when the save failed, so the popover stays open. */
  async function persist(): Promise<boolean> {
    if (!dirty) return true;
    try {
      await save.mutateAsync(draft);
      return true;
    } catch {
      return false;
    }
  }

  function handleOpenChange(next: boolean) {
    if (next) {
      setDraft(saved);
      setOpen(true);
      return;
    }
    void persist().then((ok) => {
      if (ok) setOpen(false);
    });
  }

  async function handlePreview() {
    if (!(await persist())) return;
    setOpen(false);
    onPreview(role);
  }

  function setCeiling(ceiling: Ceiling) {
    setDraft((current) => ({ ...current, ceiling }));
  }

  function toggleDepartment(id: string) {
    setDraft((current) => {
      const adding = !current.departmentIds.includes(id);
      return {
        ...current,
        departmentIds: toggled(current.departmentIds, id),
        // Removing a department also removes any exception that pointed at it.
        exceptIds: adding ? current.exceptIds : current.exceptIds.filter((item) => item !== id),
      };
    });
  }

  function toggleException(id: string) {
    setDraft((current) => ({ ...current, exceptIds: toggled(current.exceptIds, id) }));
  }

  return (
    <LabelPopover
      trigger={trigger}
      title={role.name}
      open={open}
      onOpenChange={handleOpenChange}
      testId={`profile-editor-${role.id}`}
      footer={
        <div className="flex items-center justify-between gap-3">
          <span className="text-xs text-muted-foreground">
            {dirty ? "Saves when you close" : "Saved"}
          </span>
          <Button
            variant="link"
            size="sm"
            className="h-auto p-0"
            onClick={() => void handlePreview()}
            disabled={save.isPending}
            data-testid="profile-preview"
          >
            See what this profile can open
          </Button>
        </div>
      }
    >
      <ChoiceField
        label="Ceiling"
        value={draft.ceiling}
        choices={CEILING_CHOICES}
        onChange={setCeiling}
      />
      <ChecklistGroup
        label="Departments and projects"
        testId="profile-departments"
        options={departments}
        selected={draft.departmentIds}
        onToggle={toggleDepartment}
        emptyText="No departments yet. Add them in the Departments row."
      />
      <div className="rounded-md border border-dashed px-3 py-2">
        <ChecklistGroup
          label="Except, even when a department matches"
          testId="profile-except"
          options={exceptOptions}
          selected={draft.exceptIds}
          onToggle={toggleException}
          emptyText="Choose departments above to hide some of them here."
        />
      </div>
    </LabelPopover>
  );
}
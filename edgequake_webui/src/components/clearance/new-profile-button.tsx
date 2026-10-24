"use client";

import { LabelPopover, NameForm } from "@/components/clearance/label-popover";
import { Button } from "@/components/ui/button";
import { clearanceApi } from "@/lib/clearance/api";
import { reportError } from "@/lib/clearance/errors";
import { invalidateClearanceViews } from "@/lib/clearance/use-clearance";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

/** Name a new access profile. The profile starts empty; the owner then edits it. */
export function NewProfileButton({ workspaceId }: { workspaceId: string }) {
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const create = useMutation({
    mutationFn: (name: string) => clearanceApi.createRole(workspaceId, name),
    onSuccess: () => {
      setOpen(false);
      return invalidateClearanceViews(queryClient, workspaceId);
    },
    onError: (error) => reportError(error, "Could not create that profile."),
  });

  return (
    <LabelPopover
      title="New access profile"
      open={open}
      onOpenChange={setOpen}
      align="end"
      trigger={
        <Button variant="outline" size="sm" data-testid="new-profile">
          New profile
        </Button>
      }
    >
      <NameForm
        placeholder="Profile name, such as HR reader"
        actionLabel="Create"
        onSubmit={(name) => create.mutateAsync(name)}
      />
    </LabelPopover>
  );
}

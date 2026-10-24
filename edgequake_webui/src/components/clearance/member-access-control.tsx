"use client";

import { ChecklistGroup, LabelPopover } from "@/components/clearance/label-popover";
import { Button } from "@/components/ui/button";
import { clearanceApi } from "@/lib/clearance/api";
import { reportError } from "@/lib/clearance/errors";
import { memberRolesKey, invalidateClearanceViews, useClearanceRoles, useClearanceTags } from "@/lib/clearance/use-clearance";
import { profileLine } from "@/lib/clearance/summary";
import { sameIds } from "@/lib/clearance/rules";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

/** Which access profiles one member holds. Owners and admins only. */
export function MemberAccessControl({ workspaceId, userId }: { workspaceId: string; userId: string }) {
  const queryClient = useQueryClient();
  const roles = useClearanceRoles(workspaceId, true);
  const tags = useClearanceTags(workspaceId, true);
  const held = useQuery({
    queryKey: memberRolesKey(workspaceId, userId),
    queryFn: () => clearanceApi.memberRoles(workspaceId, userId),
    enabled: Boolean(workspaceId) && Boolean(userId),
    retry: false,
  });
  const [open, setOpen] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);
  const savedIds = held.data?.role_ids ?? [];
  const profiles = roles.data ?? [];
  const heldNames = profiles.filter((role) => savedIds.includes(role.id)).map((role) => role.name);

  const save = useMutation({
    mutationFn: (roleIds: string[]) => clearanceApi.saveMemberRoles(workspaceId, userId, roleIds),
    onSuccess: () => invalidateClearanceViews(queryClient, workspaceId),
    onError: (error) => reportError(error, "Could not save this person's access."),
  });

  function handleOpenChange(next: boolean) {
    if (next) {
      setSelected(savedIds);
      setOpen(true);
      return;
    }
    if (sameIds(selected, savedIds)) {
      setOpen(false);
      return;
    }
    save
      .mutateAsync(selected)
      .then(() => setOpen(false))
      .catch(() => undefined);
  }

  const options = profiles.map((role) => ({
    id: role.id,
    label: role.name,
    hint: profileLine(role, tags.data ?? []),
  }));

  return (
    <LabelPopover
      title="Access profiles"
      open={open}
      onOpenChange={handleOpenChange}
      align="start"
      testId="member-access-editor"
      trigger={
        <Button
          variant="outline"
          size="sm"
          className="h-8 max-w-full justify-start truncate"
          data-testid="member-access-profile"
          aria-label="Edit access profiles"
        >
          {heldNames.length > 0 ? heldNames.join(", ") : "No profile"}
        </Button>
      }
    >
      <ChecklistGroup
        label="Profiles for this person"
        testId="member-access-options"
        options={options}
        selected={selected}
        onToggle={(id) =>
          setSelected((current) =>
            current.includes(id) ? current.filter((item) => item !== id) : [...current, id],
          )
        }
        emptyText="No profiles yet. Create one under Access in Settings."
      />
    </LabelPopover>
  );
}

"use client";

import { AccessProfileEditor } from "@/components/clearance/access-profile-editor";
import { NewProfileButton } from "@/components/clearance/new-profile-button";
import { VocabularyEditor } from "@/components/clearance/vocabulary-editor";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Switch } from "@/components/ui/switch";
import { SettingsNotice } from "@/components/settings/shell/settings-frame";
import { clearanceApi } from "@/lib/clearance/api";
import { reportError } from "@/lib/clearance/errors";
import { compartmentTags, documentCountLabel, profileLine } from "@/lib/clearance/summary";
import type { ClearanceSettings, SecurityRoleView } from "@/lib/clearance/types";
import {
  invalidateClearanceViews,
  useClearanceRoles,
  useClearanceSettings,
  useClearanceTags,
} from "@/lib/clearance/use-clearance";
import { useTenantStore } from "@/stores/use-tenant-store";
import { useMutation, useQueryClient, type UseQueryResult } from "@tanstack/react-query";
import { useState, type ReactNode } from "react";

const PREVIEW_LIMIT = 20;

interface PreviewState {
  roleName: string;
  titles: string[];
  count: number | null;
}

/** One setting: a title, one supporting line, and the control on the right. */
function SettingRow({
  title,
  description,
  control,
  testId,
}: {
  title: string;
  description?: string;
  control: ReactNode;
  testId?: string;
}) {
  return (
    <div className="eq-setting-row flex-nowrap px-4 py-3" data-testid={testId}>
      <div className="min-w-0">
        <div className="text-sm font-medium">{title}</div>
        {description ? (
          <div className="text-xs text-muted-foreground">{description}</div>
        ) : null}
      </div>
      <div className="shrink-0">{control}</div>
    </div>
  );
}

/**
 * What to show instead of the editors, or null when the editors can render.
 * Every non-happy path says something, so the Access screen is never blank.
 */
function accessNotice(
  settings: Pick<UseQueryResult<ClearanceSettings>, "isPending" | "isError" | "refetch">,
  canManage: boolean,
): ReactNode {
  if (settings.isPending) {
    return <SettingsNotice testId="document-labels-status">Loading access settings…</SettingsNotice>;
  }
  if (settings.isError) {
    return (
      <SettingsNotice tone="danger" testId="document-labels-status">
        <span>Could not load access settings.</span>{" "}
        <Button
          type="button"
          variant="link"
          size="sm"
          className="h-auto p-0"
          onClick={() => void settings.refetch()}
        >
          Try again
        </Button>
      </SettingsNotice>
    );
  }
  if (!canManage) {
    return (
      <SettingsNotice testId="document-labels-status">
        Only workspace owners and admins can change document access.
      </SettingsNotice>
    );
  }
  return null;
}

/** Settings → Access. Owners and admins only. */
export function DocumentLabelsCard() {
  const workspaceId = useTenantStore((state) => state.selectedWorkspaceId);
  const settings = useClearanceSettings(workspaceId);
  const canManage = settings.data?.can_manage === true;
  const tags = useClearanceTags(workspaceId, canManage);
  const roles = useClearanceRoles(workspaceId, canManage);
  const queryClient = useQueryClient();
  const [confirm, setConfirm] = useState(false);
  const [strictStart, setStrictStart] = useState(false);
  const [preview, setPreview] = useState<PreviewState | null>(null);

  const enable = useMutation({
    mutationFn: () => clearanceApi.enable(workspaceId!, strictStart),
    onSuccess: () => {
      setConfirm(false);
      return invalidateClearanceViews(queryClient, workspaceId!);
    },
    onError: (error) => reportError(error, "Could not turn on document labels."),
  });

  if (!workspaceId) {
    return <SettingsNotice testId="document-labels-status">Choose a workspace to manage document access.</SettingsNotice>;
  }
  const notice = accessNotice(settings, canManage);
  if (notice) return notice;

  const on = settings.data?.enforcement === true;
  const tagList = tags.data ?? [];
  const profiles: SecurityRoleView[] = roles.data ?? [];
  const departmentNames = compartmentTags(tagList).map((tag) => tag.value);

  async function openPreview(role: SecurityRoleView) {
    try {
      const result = await clearanceApi.simulate(workspaceId!, [role.id]);
      setPreview({
        roleName: role.name,
        titles: result.titles.slice(0, PREVIEW_LIMIT),
        count: typeof result.count === "number" ? result.count : null,
      });
    } catch (error) {
      reportError(error, "Could not load the preview for this profile.");
    }
  }

  return (
    <Card data-testid="document-labels-card">
      <CardContent className="divide-y p-0">
        <SettingRow
          title="Document labels"
          description="Members see only the documents their profile allows."
          testId="document-labels-row"
          control={
            <Switch
              checked={on}
              disabled={on || enable.isPending}
              onCheckedChange={(next) => {
                if (next) setConfirm(true);
              }}
              aria-label="Document labels"
              data-testid="document-labels-switch"
            />
          }
        />
        {!on ? (
          <SettingRow
            title="Mark existing documents as not reviewed"
            testId="strict-start-row"
            control={
              <Switch
                checked={strictStart}
                onCheckedChange={setStrictStart}
                aria-label="Mark existing documents as not reviewed"
              />
            }
          />
        ) : null}

        <section className="space-y-3 px-4 py-4" data-testid="access-profiles">
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <h4 className="text-sm font-medium">Access profiles</h4>
              <p className="text-xs text-muted-foreground">Each profile sets what its members can open.</p>
            </div>
            <NewProfileButton workspaceId={workspaceId} />
          </div>
          {roles.isPending ? (
            <p className="rounded-md border border-dashed px-3 py-4 text-center text-sm text-muted-foreground" data-testid="access-profiles-loading">
              Loading profiles…
            </p>
          ) : roles.isError ? (
            <p role="alert" className="rounded-md border border-dashed px-3 py-4 text-center text-sm text-destructive">
              Could not load profiles. Refresh the page to try again.
            </p>
          ) : profiles.length === 0 ? (
            <p className="rounded-md border border-dashed px-3 py-4 text-center text-sm text-muted-foreground">
              No profiles yet. Create one to start.
            </p>
          ) : (
            <ul className="divide-y rounded-md border">
              {profiles.map((role) => (
                <li key={role.id} className="flex items-center justify-between gap-3 px-3 py-2.5">
                  <div className="min-w-0">
                    <div className="truncate text-sm font-medium">{role.name}</div>
                    <div className="truncate text-xs text-muted-foreground">{profileLine(role, tagList)}</div>
                  </div>
                  <AccessProfileEditor
                    workspaceId={workspaceId}
                    role={role}
                    tags={tagList}
                    onPreview={openPreview}
                    trigger={
                      <Button variant="outline" size="sm" className="shrink-0" data-testid={`profile-${role.id}`}>
                        Edit
                      </Button>
                    }
                  />
                </li>
              ))}
            </ul>
          )}
        </section>

        <SettingRow
          title="Departments and projects"
          description={departmentNames.length > 0 ? departmentNames.join(", ") : "None yet"}
          testId="vocabulary-row"
          control={
            <VocabularyEditor
              workspaceId={workspaceId}
              tags={tagList}
              trigger={
                <Button variant="outline" size="sm" data-testid="clearance-vocabulary">
                  Edit
                </Button>
              }
            />
          }
        />
      </CardContent>

      <AlertDialog open={confirm} onOpenChange={setConfirm}>
        <AlertDialogContent data-testid="clearance-enable-dialog">
          <AlertDialogHeader>
            <AlertDialogTitle>Turn on document labels</AlertDialogTitle>
            <AlertDialogDescription>
              Members without an access profile will see an empty library. You will still see every document.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction data-testid="clearance-turn-on" onClick={() => enable.mutate()}>
              Turn on
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      <Sheet open={preview !== null} onOpenChange={(open) => !open && setPreview(null)}>
        <SheetContent className="sm:max-w-md" data-testid="clearance-preview-sheet">
          <SheetHeader className="border-b pb-4 pr-12">
            <SheetTitle>{preview?.roleName ?? "Preview"}</SheetTitle>
            <SheetDescription data-testid="clearance-preview-count">
              {preview ? documentCountLabel(preview.count) : ""}
            </SheetDescription>
          </SheetHeader>
          <div className="flex-1 overflow-y-auto px-4 pb-6">
            {preview && preview.titles.length > 0 ? (
              <ul className="divide-y">
                {preview.titles.map((title) => (
                  <li key={title} className="truncate py-2.5 text-sm">
                    {title}
                  </li>
                ))}
              </ul>
            ) : (
              <p className="py-6 text-center text-sm text-muted-foreground">
                No documents match this profile yet.
              </p>
            )}
          </div>
        </SheetContent>
      </Sheet>
    </Card>
  );
}

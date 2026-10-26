'use client';

/**
 * Vision LLM Settings Card (Settings → Ingestion).
 *
 * Shows and edits the workspace vision (VLM) choice through the shared `VisionModelField`.
 * Saves through `visionChoicePatch`, the same write path as the workspace page.
 *
 * @implements SPEC-040: Vision LLM workspace override for PDF extraction
 */

import { VisionModelField } from '@/components/workspace/vision-model-field';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import type { LLMSelection } from '@/components/workspace/llm-model-selector';
import { getWorkspace, updateWorkspace } from '@/lib/api/edgequake';
import { getWorkspaceVisionSelection } from '@/lib/workspace/drafts';
import { visionChoicePatch } from '@/lib/workspace/vision-choice';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Eye, Pencil, Save, X } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

export function VisionLLMSettingsCard() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();

  const [isEditing, setIsEditing] = useState(false);
  const [selectedVisionLLM, setSelectedVisionLLM] = useState<LLMSelection | undefined>(undefined);

  const { data: workspace, isLoading } = useQuery({
    queryKey: ['workspace', selectedTenantId, selectedWorkspaceId],
    queryFn: () => getWorkspace(selectedTenantId!, selectedWorkspaceId!),
    enabled: !!selectedTenantId && !!selectedWorkspaceId,
    staleTime: 60000,
    retry: 1,
  });

  const updateMutation = useMutation({
    mutationFn: () =>
      updateWorkspace(
        selectedTenantId!,
        selectedWorkspaceId!,
        visionChoicePatch(
          workspace?.llm_roles,
          selectedVisionLLM
            ? { provider: selectedVisionLLM.provider, model: selectedVisionLLM.model }
            : undefined,
        ),
      ),
    onSuccess: () => {
      toast.success(t('settings.vision.updateSuccess', 'Vision LLM configuration updated'));
      queryClient.invalidateQueries({ queryKey: ['workspace', selectedTenantId, selectedWorkspaceId] });
      setIsEditing(false);
    },
    onError: (error) => {
      toast.error(t('settings.vision.updateFailed', 'Failed to update vision LLM configuration'), {
        description: error instanceof Error ? error.message : 'Unknown error',
      });
    },
  });

  const startEditing = () => {
    setSelectedVisionLLM(getWorkspaceVisionSelection(workspace));
    setIsEditing(true);
  };

  const handleCancel = () => {
    setIsEditing(false);
    setSelectedVisionLLM(getWorkspaceVisionSelection(workspace));
  };

  if (!selectedTenantId || !selectedWorkspaceId) {
    return null;
  }

  return (
    <Card>
      <CardHeader className="pb-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Eye className="h-5 w-5 text-orange-600" />
            <CardTitle>{t('settings.vision.title', 'Vision LLM Default')}</CardTitle>
          </div>
          {!isEditing && (
            <Button
              variant="ghost"
              size="sm"
              onClick={startEditing}
              aria-label={t('common.edit', 'Edit')}
            >
              <Pencil className="h-4 w-4" />
            </Button>
          )}
        </div>
        <CardDescription>
          {t(
            'settings.vision.subtitle',
            'Default Vision LLM (VLM) for PDF page extraction — not an embedding model. Text embeddings stay on the embedding stack.',
          )}
        </CardDescription>
      </CardHeader>

      <CardContent className="space-y-4">
        {isLoading ? (
          <Skeleton className="h-14 w-full" />
        ) : workspace ? (
          <>
            <VisionModelField
              workspace={workspace}
              isEditing={isEditing}
              value={selectedVisionLLM}
              onChange={setSelectedVisionLLM}
            />
            {isEditing && (
              <div className="flex items-center gap-2 pt-2">
                <Button
                  size="sm"
                  onClick={() => updateMutation.mutate()}
                  disabled={updateMutation.isPending}
                >
                  <Save className="h-4 w-4 mr-2" />
                  {t('common.save', 'Save')}
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleCancel}
                  disabled={updateMutation.isPending}
                >
                  <X className="h-4 w-4 mr-2" />
                  {t('common.cancel', 'Cancel')}
                </Button>
              </div>
            )}
          </>
        ) : (
          <p className="text-sm text-muted-foreground">
            {t('settings.vision.noVisionModelDesc', 'Configure a vision model to enable image-aware document processing')}
          </p>
        )}
      </CardContent>
    </Card>
  );
}

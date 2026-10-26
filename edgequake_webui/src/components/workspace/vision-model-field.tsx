'use client';

/**
 * The one vision (VLM) model control (SPEC-040 / SPEC-163).
 *
 * Editing: a picker limited to vision-capable models that are eligible for the `vlm` usage.
 * Viewing: the resolved choice and where it comes from. The settings card and the workspace
 * PDF card both render this, so the two surfaces always show the same choice.
 */

import { ProviderIcon } from '@/components/providers/provider-icon';
import { Badge } from '@/components/ui/badge';
import { LLMModelSelector, type LLMSelection } from '@/components/workspace/llm-model-selector';
import { effectiveVisionFromWorkspace } from '@/lib/config/resolve-model-choice';
import type { Workspace } from '@/types/workspace';
import { useTranslation } from 'react-i18next';

export interface VisionModelFieldProps {
  workspace: Workspace;
  isEditing: boolean;
  value: LLMSelection | undefined;
  onChange: (value: LLMSelection | undefined) => void;
}

export function VisionModelField({ workspace, isEditing, value, onChange }: VisionModelFieldProps) {
  if (isEditing) {
    return (
      <LLMModelSelector value={value} onChange={onChange} filterVision usage="vlm" showUsageHint />
    );
  }
  return <VisionResolvedSummary workspace={workspace} />;
}

function VisionResolvedSummary({ workspace }: { workspace: Workspace }) {
  const { t } = useTranslation();
  const resolved = effectiveVisionFromWorkspace(workspace);
  const hasOverride = resolved.source === 'workspace';
  const resolvesTo = `${resolved.provider}/${resolved.model}`;

  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-2 p-3 bg-muted/50 rounded-lg min-h-[3.75rem]">
      <ProviderIcon providerId={resolved.provider} />
      <div className="min-w-0 flex-1 basis-40">
        <div className="font-medium truncate">
          {hasOverride
            ? resolved.model
            : t('workspace.serverDefault', 'Server Default')}
        </div>
        <div className="text-sm text-muted-foreground capitalize truncate">
          {hasOverride
            ? t('workspace.workspaceOverride', 'Workspace override')
            : t('workspace.resolvesVia', 'Resolves via {{source}}', { source: resolved.source })}
        </div>
      </div>
      <Badge
        variant="outline"
        className="ml-auto min-w-0 max-w-full shrink font-mono text-xs"
        title={`source=${resolved.source}`}
        data-testid="vision-resolves-to"
      >
        <span className="truncate">
          {t('settings.pdfParser.resolvesTo', 'Resolves to {{value}}', { value: resolvesTo })}
        </span>
      </Badge>
    </div>
  );
}

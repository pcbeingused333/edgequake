'use client';

import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { slugifyName } from '@/lib/onboarding/slug';
import type { WizardDraft } from '@/lib/onboarding/wizard-state';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface WorkspaceBasicsStepProps {
  draft: WizardDraft;
  onChange: (patch: Partial<WizardDraft>) => void;
}

export function WorkspaceBasicsStep({ draft, onChange }: WorkspaceBasicsStepProps) {
  const { t } = useTranslation();
  // The footer already explains a disabled Next; only nag once the user has typed and left.
  const [edited, setEdited] = useState(false);
  const [touched, setTouched] = useState(false);
  const nameError =
    touched && draft.workspaceName.trim().length === 0
      ? t('onboarding.workspaceNameRequired', 'Workspace name is required.')
      : null;

  const slugPreview = slugifyName(draft.workspaceSlug.trim() || draft.workspaceName);

  return (
    <div className="space-y-4" data-testid="wizard-step-workspace-basics">
      <div className="grid gap-2">
        <Label htmlFor="wizard-workspace-name">
          {t('workspace.name', 'Name')}
          <span className="text-destructive ml-0.5">*</span>
        </Label>
        <Input
          id="wizard-workspace-name"
          value={draft.workspaceName}
          onChange={(e) => {
            setEdited(true);
            onChange({ workspaceName: e.target.value });
          }}
          onBlur={() => edited && setTouched(true)}
          placeholder={t('workspace.namePlaceholderExample', 'e.g. Project Alpha')}
          data-autofocus
          autoComplete="off"
          data-wizard-enter
          aria-invalid={Boolean(nameError)}
          aria-describedby={nameError ? 'wizard-workspace-name-error' : undefined}
          data-testid="wizard-workspace-name"
        />
        {nameError ? (
          <p id="wizard-workspace-name-error" role="alert" className="text-xs text-destructive">
            {nameError}
          </p>
        ) : null}
      </div>

      <div className="grid gap-2">
        <Label htmlFor="wizard-workspace-slug">
          {t('workspace.slug', 'Slug')}{' '}
          <span className="text-muted-foreground font-normal">
            ({t('common.optional', 'optional')})
          </span>
        </Label>
        <Input
          id="wizard-workspace-slug"
          value={draft.workspaceSlug}
          onChange={(e) => onChange({ workspaceSlug: e.target.value })}
          placeholder={slugifyName(draft.workspaceName) || 'auto-generated'}
          autoComplete="off"
          aria-describedby="wizard-workspace-slug-hint"
          data-wizard-enter
          data-testid="wizard-workspace-slug"
        />
        <p
          id="wizard-workspace-slug-hint"
          className="text-xs text-muted-foreground"
          data-testid="wizard-workspace-slug-preview"
        >
          {slugPreview
            ? t('workspace.slugPreview', 'Used in URLs: /query?workspace={{slug}}', {
                slug: slugPreview,
              })
            : t('workspace.slugHintEmpty', 'Generated from the name. Used in URLs.')}
        </p>
      </div>

      <div className="grid gap-2">
        <Label htmlFor="wizard-workspace-description">
          {t('workspace.description', 'Description')}
        </Label>
        <Textarea
          id="wizard-workspace-description"
          value={draft.workspaceDescription}
          onChange={(e) => onChange({ workspaceDescription: e.target.value })}
          placeholder={t('workspace.descriptionPlaceholder', 'A brief description…')}
          rows={2}
          data-testid="wizard-workspace-description"
        />
      </div>
    </div>
  );
}

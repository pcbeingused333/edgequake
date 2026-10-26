import type { WizardStepId } from '@/lib/onboarding/wizard-state';

export const STEP_META: Record<
  WizardStepId,
  { titleKey: string; title: string; descriptionKey: string; description: string }
> = {
  admin: {
    titleKey: 'onboarding.adminTitle',
    title: 'Create admin account',
    descriptionKey: 'onboarding.adminSubtitle',
    description: 'This password secures your EdgeQuake instance.',
  },
  'tenant-basics': {
    titleKey: 'onboarding.tenantTitle',
    title: 'Name your organization',
    descriptionKey: 'onboarding.tenantSubtitle',
    description: 'Organizations isolate workspaces and data.',
  },
  models: {
    titleKey: 'onboarding.modelsTitle',
    title: 'Confirm AI models',
    descriptionKey: 'onboarding.modelsSubtitle',
    description: 'Inherited defaults apply unless you override.',
  },
  'workspace-basics': {
    titleKey: 'onboarding.workspaceTitle',
    title: 'Name your workspace',
    descriptionKey: 'onboarding.workspaceSubtitle',
    description: 'Documents and extracted knowledge live here.',
  },
  'document-parsing': {
    titleKey: 'onboarding.documentParsingTitle',
    title: 'Document parsing',
    descriptionKey: 'onboarding.documentParsingSubtitle',
    description: 'Choose how PDFs are converted to text.',
  },
  chunking: {
    titleKey: 'onboarding.chunkingTitle',
    title: 'Chunking',
    descriptionKey: 'onboarding.chunkingSubtitle',
    description: 'How documents are split before entity extraction.',
  },
  'extract-budget': {
    titleKey: 'onboarding.extractBudgetTitle',
    title: 'Extract budget',
    descriptionKey: 'onboarding.extractBudgetSubtitle',
    description: 'Per-response entity and record caps for LLM extraction.',
  },
  extraction: {
    titleKey: 'onboarding.extractionTitle',
    title: 'Extraction preferences',
    descriptionKey: 'onboarding.extractionSubtitle',
    description: 'Language and knowledge-graph schema for extraction.',
  },
  review: {
    titleKey: 'onboarding.reviewTitle',
    title: 'Review and create',
    descriptionKey: 'onboarding.reviewSubtitle',
    description: 'Nothing is saved until you confirm.',
  },
};

/** Reconfigure uses Apply-oriented review copy (LAW-101-12). */
export const RECONFIGURE_REVIEW_META = {
  titleKey: 'onboarding.reconfigureReviewTitle',
  title: 'Review and apply',
  descriptionKey: 'onboarding.reconfigureReviewSubtitle',
  description: 'Confirm changes. Rebuild may be required for existing docs.',
} as const;

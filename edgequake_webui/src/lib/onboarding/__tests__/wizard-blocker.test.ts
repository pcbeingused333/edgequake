import { describe, expect, it } from 'vitest';
import { describeBlocker, type BlockerTranslate } from '../wizard-blocker-message';
import { hasAnyModelPick } from '../wizard-picks';
import { EMPTY_WIZARD_DRAFT, canProceed, wizardBlocker } from '../wizard-state';

const t: BlockerTranslate = (_key, fallback, options) =>
  fallback.replace(/\{\{(\w+)\}\}/g, (_m, k: string) => options?.[k] ?? '');
const open = { hasConfiguredDefaults: true, advancedModelsValid: true };

describe('wizardBlocker', () => {
  it('names the first reason a step cannot advance', () => {
    expect(wizardBlocker('workspace-basics', EMPTY_WIZARD_DRAFT)).toBe('workspace_name_required');
    expect(wizardBlocker('workspace-basics', { ...EMPTY_WIZARD_DRAFT, workspaceName: 'A' })).toBeNull();
    expect(wizardBlocker('tenant-basics', EMPTY_WIZARD_DRAFT)).toBe('tenant_name_required');
  });

  it('ranks eligibility above completeness on the models step', () => {
    const custom = { ...EMPTY_WIZARD_DRAFT, useServerDefaults: false };
    const gate = { ...open, advancedModelsValid: false };
    expect(wizardBlocker('models', custom, { ...gate, eligibilityOk: false })).toBe('models_not_eligible');
    expect(wizardBlocker('models', custom, gate)).toBe('models_incomplete');
    expect(wizardBlocker('models', EMPTY_WIZARD_DRAFT, { ...open, hasConfiguredDefaults: false })).toBe(
      'defaults_incomplete',
    );
  });

  it('review needs eligibility and at least one change when reconfiguring', () => {
    expect(wizardBlocker('review', EMPTY_WIZARD_DRAFT, { ...open, hasConfigChanges: false })).toBe('no_changes');
    expect(wizardBlocker('review', EMPTY_WIZARD_DRAFT, { ...open, eligibilityOk: false })).toBe(
      'models_not_eligible',
    );
    expect(wizardBlocker('review', EMPTY_WIZARD_DRAFT, open)).toBeNull();
  });

  it('canProceed is exactly "no blocker"', () => {
    expect(canProceed('workspace-basics', EMPTY_WIZARD_DRAFT)).toBe(false);
    expect(canProceed('extraction', EMPTY_WIZARD_DRAFT)).toBe(true);
  });
});

describe('describeBlocker', () => {
  it('is silent when nothing blocks', () => {
    expect(describeBlocker(null, t)).toBeNull();
  });

  it('asks for one pick', () => {
    expect(describeBlocker('models_incomplete', t)).toBe(
      'Pick at least one model to continue.',
    );
  });

  it('names the refused role, or that nothing is offered', () => {
    const verdicts = { llm: 'ok', embedding: 'dimension_missing', vision: 'ok' } as const;
    expect(describeBlocker('models_not_eligible', t, { verdicts })).toBe(
      'Choose another model for: Embedding.',
    );
    expect(describeBlocker('models_not_eligible', t, { verdicts, scope: 'workspace' })).toBe(
      'Turn on a provider for this workspace, or choose another model for: Embedding.',
    );
    expect(describeBlocker('models_not_eligible', t, { nothingOffered: true })).toMatch(
      /this organization/,
    );
    expect(describeBlocker('models_not_eligible', t, { nothingOffered: true, scope: 'workspace' })).toBe(
      'No provider is offered to this workspace yet.',
    );
  });

  it('every code has a sentence', () => {
    for (const code of [
      'admin_invalid',
      'tenant_name_required',
      'workspace_name_required',
      'models_not_eligible',
      'defaults_incomplete',
      'models_incomplete',
      'chunking_invalid',
      'extract_budget_invalid',
      'no_changes',
    ] as const) {
      expect(describeBlocker(code, t)).toBeTruthy();
    }
  });
});

describe('hasAnyModelPick', () => {
  it('accepts a partial override, rejects an empty one', () => {
    const none = { llm: undefined, embedding: { provider: 'mistral' }, vision: undefined };
    expect(hasAnyModelPick(none)).toBe(false);
    expect(hasAnyModelPick({ ...none, vision: { provider: 'openai', model: 'gpt-5.2' } })).toBe(true);
  });
});

import type { WizardRole } from '@/lib/onboarding/defaults-eligibility';

function scrollParentOf(node: HTMLElement): HTMLElement | null {
  let parent = node.parentElement;
  while (parent) {
    const overflow = getComputedStyle(parent).overflowY;
    if (overflow === 'auto' || overflow === 'scroll') return parent;
    parent = parent.parentElement;
  }
  return null;
}

/**
 * Bring one role's picker into the wizard scrollport and return its provider
 * trigger. Scroll is applied on the dialog body directly so it finishes before
 * the menu opens — scrollIntoView would keep moving the portaled menu.
 */
export function revealWizardRolePicker(role: WizardRole): HTMLButtonElement | null {
  const root = document.querySelector(`[data-wizard-model-role="${role}"]`);
  if (!(root instanceof HTMLElement)) return null;
  const trigger = root.querySelector('[data-testid="model-picker-provider-trigger"]');
  if (!(trigger instanceof HTMLButtonElement) || trigger.disabled) return null;
  const parent = scrollParentOf(root);
  if (parent) {
    const top = root.getBoundingClientRect().top - parent.getBoundingClientRect().top + parent.scrollTop;
    parent.scrollTop = Math.max(0, top - (parent.clientHeight - root.offsetHeight) / 2);
  }
  return trigger;
}

export function openPickerTrigger(trigger: HTMLButtonElement): void {
  if (trigger.getAttribute('aria-expanded') !== 'true') trigger.click();
}

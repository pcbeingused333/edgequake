import { describe, expect, it } from 'vitest';
import { openPickerTrigger, revealWizardRolePicker } from '../wizard-role-picker';

describe('revealWizardRolePicker', () => {
  it('scrolls the dialog body to the named role and opens its provider menu once', () => {
    document.body.innerHTML = `
      <div id="scroll" style="overflow-y: auto">
        <section data-wizard-model-role="embedding">
          <button type="button" data-testid="model-picker-provider-trigger" aria-expanded="false">Provider</button>
        </section>
      </div>
    `;
    const scroll = document.getElementById('scroll') as HTMLElement;
    Object.defineProperty(scroll, 'clientHeight', { value: 100 });
    const section = scroll.querySelector('section') as HTMLElement;
    Object.defineProperty(section, 'offsetHeight', { value: 40 });
    section.getBoundingClientRect = () => ({ top: 240, bottom: 280, left: 0, right: 0, width: 0, height: 40, x: 0, y: 240, toJSON() {} });
    scroll.getBoundingClientRect = () => ({ top: 0, bottom: 100, left: 0, right: 0, width: 0, height: 100, x: 0, y: 0, toJSON() {} });

    const trigger = revealWizardRolePicker('embedding');
    expect(trigger).toBeInstanceOf(HTMLButtonElement);
    expect(scroll.scrollTop).toBe(210);
    let clicks = 0;
    trigger!.onclick = () => {
      clicks += 1;
      trigger!.setAttribute('aria-expanded', 'true');
    };
    openPickerTrigger(trigger!);
    openPickerTrigger(trigger!);
    expect(clicks).toBe(1);
    expect(revealWizardRolePicker('vision')).toBeNull();
  });
});

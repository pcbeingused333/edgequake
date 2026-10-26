import { describe, expect, it } from 'vitest';
import { isPortaledChoiceMenu } from '../portaled-menu';

describe('isPortaledChoiceMenu', () => {
  it('recognises a click inside a portaled picker, including after it leaves the document', () => {
    const wrapper = document.createElement('div');
    wrapper.setAttribute('data-radix-popper-content-wrapper', '');
    const content = document.createElement('div');
    content.setAttribute('data-slot', 'popover-content');
    const item = document.createElement('div');
    content.append(item);
    wrapper.append(content);
    document.body.append(wrapper);

    expect(isPortaledChoiceMenu(item)).toBe(true);
    wrapper.remove();
    expect(isPortaledChoiceMenu(item)).toBe(true);
    expect(isPortaledChoiceMenu(document.body)).toBe(false);
    expect(isPortaledChoiceMenu(null)).toBe(false);
  });
});

/**
 * Menus inside a wizard are portaled to document.body. A pointer event on one
 * is "outside" the dialog, but it is still the user's selection.
 */

const PORTALED_MENU =
  '[data-slot="popover-content"], [data-radix-popper-content-wrapper], [data-slot="select-content"]';

export function isPortaledChoiceMenu(target: EventTarget | null): boolean {
  return target instanceof Element && target.closest(PORTALED_MENU) !== null;
}

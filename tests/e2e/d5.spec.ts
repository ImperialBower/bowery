// D5: "to call $10,000" and the button shown after the hand was over. The
// last frame has nobody to act and no action buttons.
import { expect, test } from '@playwright/test';
import { allInLines } from './helpers';

test('d5: nothing to act on after the hand', async ({ page }) => {
  for (const [line, shown] of await allInLines(page)) {
    const last = shown.at(-1)!;
    expect(last.toAct, line).toBe(0);
    for (const action of ['Fold', 'Check', 'All in', 'Bet', 'Raise to']) {
      expect(last.buttons, line).not.toContain(action);
    }
    expect(last.buttons.some((name) => name.startsWith('Call')), line).toBe(false);
  }
});

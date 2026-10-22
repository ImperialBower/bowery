// D4: stale bets on the felt after the award. The last frame has an empty pot
// and no bets.
import { expect, test } from '@playwright/test';
import { allInLines } from './helpers';

test('d4: no bets and no pot after the award', async ({ page }) => {
  for (const [line, shown] of await allInLines(page)) {
    const last = shown.at(-1)!;
    expect(last.caption, line).toBe('Short wins 400');
    expect(last.pot, line).toBe('POT 0');
    expect(last.bets, line).toEqual([]);
  }
});

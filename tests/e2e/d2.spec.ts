// D2: the all-in hole cards were never turned up. In seat 1's view, seat 0's
// cards are down until the showdown and up from it.
import { expect, test } from '@playwright/test';
import { allInLines } from './helpers';

test('d2: cards turn up at the showdown', async ({ page }) => {
  for (const [line, shown] of await allInLines(page)) {
    const showdown = shown.findIndex((frame) => frame.caption === 'Showdown');
    expect(showdown, line).toBeGreaterThan(0);
    expect(shown[showdown - 1].backs['0'], line).toBe(2);
    expect(shown[showdown].cards['0'], line).toEqual(['7♦', '2♣']);
    expect(shown[showdown].cards['1'], line).toEqual(['A♠', 'A♥']);
  }
});

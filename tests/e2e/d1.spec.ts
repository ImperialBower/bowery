// D1: all in preflop, the board stayed five empty slots. Each street must
// be dealt in its own frame.
import { expect, test } from '@playwright/test';
import { allInLines } from './helpers';

test('d1: every street is its own frame', async ({ page }) => {
  for (const [line, shown] of await allInLines(page)) {
    const deals = shown.filter((frame) => /^(Flop|Turn|River) · /.test(frame.caption));
    expect(deals.map((frame) => frame.board.length), line).toEqual([3, 4, 5]);
    expect(shown.at(-1)!.board, line).toEqual(['K♠', 'K♣', '9♦', '8♠', '4♥']);
  }
});

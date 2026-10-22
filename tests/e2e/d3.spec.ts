// D3: "abc won $20,250" while the pot read $10,250. The win is the pot
// shown at the showdown; the uncalled 800 goes back and is not a win.
import { expect, test } from '@playwright/test';
import { allInLines } from './helpers';

test('d3: the award is the pot, the return is not a win', async ({ page }) => {
  for (const [line, shown] of await allInLines(page)) {
    const captions = shown.map((frame) => frame.caption);
    expect(captions, line).toContain('Deep takes back 800');
    expect(captions.filter((caption) => caption.includes('wins')), line).toEqual(['Short wins 400']);
    expect(shown.find((frame) => frame.caption === 'Showdown')!.pot, line).toBe('POT 400');
  }
});

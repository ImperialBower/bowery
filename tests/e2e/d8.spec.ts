// D8: "POT $0" while $6,550 was in front of players. The pot shown is the
// pot plus every live bet, and the bets are shown at their seats.
import { expect, test } from '@playwright/test';
import { ready, snapshot } from './helpers';

test('d8: the pot counts the bets in front of the seats', async ({ page }) => {
  await page.goto('/e2e/raise.html');
  await ready(page, 'raise');

  const shown = await snapshot(page, 'raise');

  expect(shown.caption).toBe('Preflop · Daniel Negreanu raises to 5,000');
  // 8 antes of 100, the blinds 300 and 600, Hansen's 2,100, Negreanu's 5,000.
  expect(shown.pot).toBe('POT 8,800');
  expect(shown.bets).toEqual(expect.arrayContaining(['2,100', '5,000']));
});

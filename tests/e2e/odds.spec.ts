import { expect, test } from '@playwright/test';
import { ready, snapshot } from './helpers';

test('odds: percents show, the bar lists the outs at the turn, % hides them', async ({ page }) => {
  await page.goto('/e2e/outs.html');
  await ready(page, 'outs');
  const bar = page.locator('#outs .bar');
  // Preflop: a percent per live seat, no value asserted (sampled, per target).
  let shown = await snapshot(page, 'outs');
  expect(shown.odds).toHaveLength(3);
  expect(shown.outs).toEqual([]);
  // To the turn: exact numbers from here on.
  for (let step = 0; step < 100 && !/^Turn · /.test(shown.caption); step += 1) {
    await bar.getByRole('button', { name: 'next' }).click();
    shown = await snapshot(page, 'outs');
  }
  expect(shown.caption).toMatch(/^Turn · /);
  expect(shown.odds).toEqual(['17%', '83%']);
  expect(shown.outs).toEqual(['T♥', 'T♦', 'T♣', '5♠', '5♥', '5♦', '5♣']);
  const percent = bar.getByRole('button', { name: 'odds' });
  await expect(percent).toHaveAttribute('aria-pressed', 'true');
  // % off hides both; % on brings them back; the eye to seat view hides the
  // button and the eye to all brings that back too.
  await percent.click();
  shown = await snapshot(page, 'outs');
  expect(shown.odds).toEqual([]);
  expect(shown.outs).toEqual([]);
  await expect(percent).toHaveAttribute('aria-pressed', 'false');
  await percent.click();
  shown = await snapshot(page, 'outs');
  expect(shown.odds).toHaveLength(2);
  expect(shown.outs).toHaveLength(7);
  await expect(percent).toHaveAttribute('aria-pressed', 'true');
  // The eye has no aria-label (its text is "👁 all"), so find it by its control.
  const eye = bar.locator('[data-control="view"]');
  await eye.click();
  shown = await snapshot(page, 'outs');
  expect(shown.buttons).not.toContain('odds');
  await eye.click();
  shown = await snapshot(page, 'outs');
  expect(shown.buttons).toContain('odds');
});

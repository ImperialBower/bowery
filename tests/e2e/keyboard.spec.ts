// The arrow keys seek moments when the table has focus, and only then: in
// the seat picker or a bet box they belong to the control.
import { expect, test } from '@playwright/test';
import { ready, snapshot } from './helpers';

test('keyboard: arrows seek from the table', async ({ page }) => {
  await page.goto('/e2e/all-in.html');
  await ready(page, 'on');
  const before = (await snapshot(page, 'on')).moment;

  await page.locator('#on').focus();
  await page.keyboard.press('ArrowRight');

  expect((await snapshot(page, 'on')).moment).not.toBe(before);
});

test('keyboard: arrows in the seat picker do not seek', async ({ page }) => {
  await page.goto('/e2e/all-in.html');
  await ready(page, 'off');
  const before = (await snapshot(page, 'off')).moment;

  await page.locator('#off .bar select').focus();
  for (const key of ['ArrowRight', 'ArrowLeft', 'End', 'Home']) {
    await page.keyboard.press(key);
  }

  expect((await snapshot(page, 'off')).moment).toBe(before);
});

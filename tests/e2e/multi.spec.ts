// 5b: three tables on one page are three games. Act in one; the other two
// do not change.
import { expect, test } from '@playwright/test';
import { ready, snapshot } from './helpers';

const svgOf = (page: import('@playwright/test').Page, id: string) =>
  page.evaluate((id) => document.getElementById(id)!.shadowRoot!.querySelector('svg')!.outerHTML, id);

test('multi: stepping one replay leaves the others', async ({ page }) => {
  await page.goto('/e2e/multi.html');
  for (const id of ['r1', 'r2', 'r3']) await ready(page, id);
  const before = [await svgOf(page, 'r2'), await svgOf(page, 'r3')];

  await page.locator('#r1 .bar').getByRole('button', { name: 'next' }).click();

  expect((await snapshot(page, 'r1')).moment).toBe('2/25');
  expect([await svgOf(page, 'r2'), await svgOf(page, 'r3')]).toEqual(before);
});

test('multi: acting at one play table leaves the others', async ({ page }) => {
  await page.goto('/e2e/multi.html');
  for (const id of ['p1', 'p2', 'p3']) {
    await ready(page, id);
    await expect(page.locator(`#${id} .bar`).getByRole('button', { name: 'Fold' })).toBeVisible();
  }
  const before = [await svgOf(page, 'p2'), await svgOf(page, 'p3')];

  await page.locator('#p1 .bar').getByRole('button', { name: 'Fold' }).click();
  await expect(page.locator('#p1 .bar').getByRole('button', { name: 'Next hand' })).toBeVisible();

  expect((await snapshot(page, 'p1')).caption).toBe('Robo wins 100');
  expect([await svgOf(page, 'p2'), await svgOf(page, 'p3')]).toEqual(before);
});

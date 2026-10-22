// 5d: with script off, or from file://, the fallback picture stays and
// nothing throws.
import path from 'path';
import { pathToFileURL } from 'url';
import { expect, test } from '@playwright/test';

const pictures = ['the-hand-flop.svg', 'the-hand-spotlight.svg', 'the-hand-play.svg'];

test.describe('script off', () => {
  test.use({ javaScriptEnabled: false });

  test('fallback: the pictures stay', async ({ page }) => {
    await page.goto('/index.html');
    for (const picture of pictures) {
      const image = page.locator(`bowery-table img[src="${picture}"]`);
      await expect(image).toBeVisible();
      expect(await image.evaluate((element: HTMLImageElement) => element.naturalWidth)).toBeGreaterThan(0);
    }
  });
});

test('fallback: file:// keeps the pictures and throws nothing', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(String(error)));

  await page.goto(pathToFileURL(path.join(__dirname, '..', '..', 'site', 'index.html')).href);
  await page.waitForTimeout(1_000);

  for (const picture of pictures) {
    await expect(page.locator(`bowery-table img[src="${picture}"]`)).toBeVisible();
  }
  expect(errors).toEqual([]);
});

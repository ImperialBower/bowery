// 5c: every request goes to the page's own origin: no fonts, no CDN, no
// analytics.
import { expect, test } from '@playwright/test';
import { ready } from './helpers';

test('origin: the demo asks nothing of any other origin', async ({ page, baseURL }) => {
  const origins = new Set<string>();
  page.on('request', (request) => origins.add(new URL(request.url()).origin));

  await page.goto('/index.html');
  for (const id of ['replay', 'interactive', 'play', 'nine']) await ready(page, id);
  await page.locator('#replay .bar').getByRole('button', { name: 'next' }).click();
  await page.waitForTimeout(500);

  expect([...origins]).toEqual([new URL(baseURL!).origin]);
});

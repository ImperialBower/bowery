// The demo's guide pages, one per table: each table starts, the demo links
// to each guide, and the code a guide shows is the config its table runs.
import { expect, test } from '@playwright/test';
import { ready } from './helpers';

const guides = ['replay', 'interactive', 'play', 'nine'];

test('guide: the demo links every guide', async ({ page }) => {
  await page.goto('/index.html');
  for (const name of guides) {
    const link = page.locator(`a.guide[href="guide/${name}.html"]`);
    await expect(link).toHaveCount(1);
    const response = await page.request.get(`/guide/${name}.html`);
    expect(response.ok()).toBe(true);
  }
});

for (const name of guides) {
  test(`guide: ${name}'s table starts and asks nothing of any other origin`, async ({ page, baseURL }) => {
    const origins = new Set<string>();
    page.on('request', (request) => origins.add(new URL(request.url()).origin));

    await page.goto(`/guide/${name}.html`);
    await ready(page, name);
    await expect(page.locator('#log')).toContainText('bowery:ready');
    expect([...origins]).toEqual([new URL(baseURL!).origin]);
  });
}

// A whole config shown on a guide is the file its table loads, byte for byte.
// An abridged one (data-abridged) is the file's lines in order, with `# …`
// comment lines standing for what is left out.
test('guide: the configs shown are the configs run', async ({ page }) => {
  for (const name of guides) {
    await page.goto(`/guide/${name}.html`);
    for (const block of await page.locator('.code[data-config]').all()) {
      const shown = await block.locator('pre').textContent();
      const file = await (await page.request.get(`/guide/${await block.getAttribute('data-config')}`)).text();
      if ((await block.getAttribute('data-abridged')) === null) {
        expect(shown).toBe(file);
        continue;
      }
      const lines = file.split('\n');
      let at = 0;
      for (const line of shown!.split('\n')) {
        if (line.trim().startsWith('# …')) continue;
        const found = lines.indexOf(line, at);
        expect(found, `"${line}" in ${name}'s abridged config`).toBeGreaterThanOrEqual(at);
        at = found + 1;
      }
    }
  }
});

test('guide: the inline config shown is the one in the table', async ({ page }) => {
  await page.goto('/guide/nine.html');
  const shown = await page.locator('.code[data-inline="nine"] pre').textContent();
  const running = await page.locator('#nine script[type="application/yaml"]').textContent();
  expect(running!.trim().length).toBeGreaterThan(0);
  expect(shown).toContain(running!);
});

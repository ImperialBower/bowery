// D6: "GOES ALL-IN $9900" printed over joker's name. Nine seats, a widest
// name and a nine-digit stack, at 680 px and 360 px: every frame of three
// hands is checked.
import { expect, test } from '@playwright/test';
import { checked, installChecker, playHands } from './checker';

for (const id of ['ring', 'spotlight', 'phone']) {
  test(`d6: no text covers another seat (${id})`, async ({ page }) => {
    await installChecker(page);
    await page.goto('/e2e/nine.html');
    await page.locator(`#${id}`).scrollIntoViewIfNeeded();

    await playHands(page, id, 3);

    const result = await checked(page, id);
    expect(result.frames).toBeGreaterThan(30);
    expect(result.overlaps).toEqual([]);
  });
}

// The first frame too: before bowery.css arrives the element is inline, and
// was once measured at 0 and drawn as a 680 px ring.
test('d6: a phone-width ring draws as ledger from the first frame', async ({ page }) => {
  await installChecker(page);
  await page.goto('/e2e/nine.html');
  await page.locator('#phone').scrollIntoViewIfNeeded();
  await expect(page.locator('#phone svg')).toHaveAttribute('data-layout', 'ledger', { timeout: 20_000 });

  expect((await checked(page, 'phone')).first).toBe('ledger');
});

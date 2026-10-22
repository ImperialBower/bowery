// D7: FOLD bubbles drawn on top of loose_aggressive's cards. A folded seat
// shows no cards, and nothing is drawn over a seat, at any frame.
import { expect, test } from '@playwright/test';
import { checked, installChecker, playHands } from './checker';

test('d7: folded seats have no cards and nothing covers a seat', async ({ page }) => {
  await installChecker(page);
  await page.goto('/e2e/nine.html');

  await playHands(page, 'ring', 3);

  const result = await checked(page, 'ring');
  expect(result.foldedFrames).toBeGreaterThan(10);
  expect(result.foldedCards).toBe(0);
  expect(result.overlaps).toEqual([]);
});

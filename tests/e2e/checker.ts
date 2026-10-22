// D6-D7: no two things drawn cover each other, at every frame. The check
// runs in the page on each `bowery:moment`, so a frame the bots pass through
// in 25 ms is checked as surely as a slow one. It measures with the
// browser's own fonts: every seat, bet slot, the board, the pot, the
// caption, the street label and each line of the folded strip.
import { expect, type Page } from '@playwright/test';

export interface Checked {
  /** The layout of the first frame drawn. */
  first: string;
  frames: number;
  foldedFrames: number;
  foldedCards: number;
  overlaps: string[];
}

export async function installChecker(page: Page): Promise<void> {
  await page.addInitScript(() => {
    type Box = { name: string; left: number; top: number; right: number; bottom: number };
    const results: Record<string, { first: string; frames: number; foldedFrames: number; foldedCards: number; overlaps: string[] }> = {};
    (window as any).__checked = results;
    const union = (name: string, parts: Element[]): Box | null => {
      const rects = parts.map((part) => part.getBoundingClientRect()).filter((rect) => rect.width > 0 && rect.height > 0);
      if (rects.length === 0) return null;
      return {
        name,
        left: Math.min(...rects.map((rect) => rect.left)),
        top: Math.min(...rects.map((rect) => rect.top)),
        right: Math.max(...rects.map((rect) => rect.right)),
        bottom: Math.max(...rects.map((rect) => rect.bottom)),
      };
    };
    document.addEventListener('bowery:moment', (event) => {
      const host = event.target as HTMLElement;
      const svg = host.shadowRoot!.querySelector('svg')!;
      const result = (results[host.id] ??= { first: svg.dataset.layout!, frames: 0, foldedFrames: 0, foldedCards: 0, overlaps: [] });
      result.frames += 1;
      const folded = svg.querySelectorAll('g.seat.folded');
      if (folded.length > 0) result.foldedFrames += 1;
      result.foldedCards += svg.querySelectorAll('g.seat.folded g.card').length;
      const boxes: Box[] = [];
      const add = (box: Box | null) => box && boxes.push(box);
      // A seat's ink: its plate outline in ring and spotlight; its text and
      // cards in ledger, where the row's rule and highlight span the width.
      for (const seat of svg.querySelectorAll('g.seat')) {
        const ink = seat.classList.contains('row')
          ? [...seat.querySelectorAll(':scope > text, :scope > g.card')]
          : [...seat.querySelectorAll(':scope > rect.plate, :scope > text, :scope > g.card')];
        add(union(`seat ${(seat as SVGGElement).dataset.seat}`, ink));
      }
      for (const bet of svg.querySelectorAll('g.bet')) add(union(`bet ${(bet as SVGGElement).dataset.seat}`, [bet]));
      for (const fold of svg.querySelectorAll('text.bet')) add(union('fold', [fold]));
      add(union('board', [svg.querySelector('g.board')!]));
      for (const name of ['pot', 'caption', 'street']) {
        const element = svg.querySelector(`text.${name}`);
        if (element) add(union(name, [element]));
      }
      svg.querySelectorAll('text.strip').forEach((line, index) => add(union(`strip ${index}`, [line])));
      const where = `${svg.dataset.layout} ${svg.dataset.moment}`;
      const slack = 0.5;
      for (let first = 0; first < boxes.length; first += 1) {
        for (let second = first + 1; second < boxes.length; second += 1) {
          const a = boxes[first];
          const b = boxes[second];
          if (a.left < b.right - slack && b.left < a.right - slack && a.top < b.bottom - slack && b.top < a.bottom - slack) {
            result.overlaps.push(`${where}: ${a.name} covers ${b.name}`);
          }
        }
      }
      const edge = svg.getBoundingClientRect();
      for (const box of boxes) {
        if (box.left < edge.left - slack || box.right > edge.right + slack) result.overlaps.push(`${where}: ${box.name} leaves the table`);
      }
    });
  });
}

/** Plays `hands` hands at table `id` to their end, pressing Next hand between. */
export async function playHands(page: Page, id: string, hands: number): Promise<void> {
  const table = page.locator(`#${id}`);
  for (let hand = 1; hand <= hands; hand += 1) {
    await expect(table).toHaveAttribute('data-over', '', { timeout: 30_000 });
    if (hand === hands) return;
    await table.evaluate((element) => element.removeAttribute('data-over'));
    await table.locator('.bar').getByRole('button', { name: 'Next hand' }).click();
  }
}

export async function checked(page: Page, id: string): Promise<Checked> {
  return page.evaluate((id) => (window as any).__checked[id], id);
}

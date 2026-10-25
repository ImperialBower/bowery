// Reading a <bowery-table> from the outside: what its SVG and its bar show.
import { expect, type Page } from '@playwright/test';

/** What one frame shows, read from the element's shadow DOM. */
export interface Snapshot {
  moment: string;
  caption: string;
  pot: string;
  board: string[];
  /** Face-up hole cards by seat. */
  cards: Record<string, string[]>;
  /** Face-down hole cards by seat. */
  backs: Record<string, number>;
  /** Text in each bet slot. */
  bets: string[];
  toAct: number;
  /** Every percent on the plates, in document order. */
  odds: string[];
  /** The cards on the outs bar, in order. */
  outs: string[];
  buttons: string[];
}

export async function ready(page: Page, id: string): Promise<void> {
  await page.locator(`#${id}`).scrollIntoViewIfNeeded();
  await page.waitForFunction((id) => !!document.getElementById(id)?.shadowRoot?.querySelector('svg'), id, {
    timeout: 20_000,
  });
}

export async function snapshot(page: Page, id: string): Promise<Snapshot> {
  return page.evaluate((id) => {
    const root = document.getElementById(id)!.shadowRoot!;
    const svg = root.querySelector('svg')!;
    const text = (selector: string) => svg.querySelector(selector)?.textContent ?? '';
    const cards: Record<string, string[]> = {};
    const backs: Record<string, number> = {};
    for (const seat of svg.querySelectorAll('g.seat')) {
      const number = (seat as SVGGElement).dataset.seat!;
      cards[number] = [...seat.querySelectorAll('[data-card]')].map((card) => (card as SVGGElement).dataset.card!);
      backs[number] = seat.querySelectorAll('g.card.back').length;
    }
    return {
      moment: svg.dataset.moment!,
      caption: text('text.caption'),
      pot: text('text.pot'),
      board: [...svg.querySelectorAll('g.board [data-card]')].map((card) => (card as SVGGElement).dataset.card!),
      cards,
      backs,
      bets: [...svg.querySelectorAll('g.bet')].map((bet) => bet.textContent ?? ''),
      toAct: svg.querySelectorAll('g.seat.to-act').length,
      odds: [...svg.querySelectorAll('text.odds')].map((odds) => odds.textContent ?? ''),
      outs: [...svg.querySelectorAll('g.outs [data-card]')].map((card) => (card as SVGGElement).dataset.card!),
      buttons: [...root.querySelectorAll('.bar button')].map((button) => button.getAttribute('aria-label') ?? button.textContent ?? ''),
    };
  }, id);
}

/** Every frame of the table's line, from the first: ⏮, then ▶ to the end. */
export async function frames(page: Page, id: string): Promise<Snapshot[]> {
  const bar = page.locator(`#${id} .bar`);
  await bar.getByRole('button', { name: 'first' }).click();
  const shown: Snapshot[] = [await snapshot(page, id)];
  for (let step = 0; step < 100; step += 1) {
    const [at, of] = shown[shown.length - 1].moment.split('/');
    if (at === of) return shown;
    await bar.getByRole('button', { name: 'next' }).click();
    shown.push(await snapshot(page, id));
  }
  throw new Error(`${id}: no end in 100 frames`);
}

/** The all-in hand, on the record and off it (seat 1 calls the shove). */
export async function allInLines(page: Page): Promise<[string, Snapshot[]][]> {
  await page.goto('/e2e/all-in.html');
  await ready(page, 'on');
  await ready(page, 'off');
  const off = page.locator('#off .bar');
  await off.locator('select').selectOption('1');
  await off.getByRole('button', { name: 'Take this seat' }).click();
  await off.getByRole('button', { name: /^Call/ }).click();
  // Off the record now: the game deals the run-out and pays, by itself.
  await expect(off.getByRole('button', { name: 'Back to the record' })).toBeVisible();
  await expect(page.locator('#off')).toHaveAttribute('data-over', '', { timeout: 20_000 });
  return [
    ['on the record', await frames(page, 'on')],
    ['off the record', await frames(page, 'off')],
  ];
}

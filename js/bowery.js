// <bowery-table>: one poker table control, configured not compiled.
//
// Usage:
//   <script type="module" src="bowery/bowery.js"></script>
//   <bowery-table base="bowery/">
//     <script type="application/yaml">mode: replay …</script>
//     <svg …>the fallback picture</svg>
//   </bowery-table>
//
// Nothing loads until the element is within a screen of the viewport. The
// first element instantiates the wasm module; every later one reuses it and
// builds its own BoweryTable, so tables on a page are independent. The
// fallback picture stays if the config, the import or the init fails, and a
// config's problems are printed under it. Every request goes to `base`
// (default: this module's directory) or to `src`, both on the page's origin.
// Contract: bower docs/EPIC-20_Bowery.md § <bowery-table>.

const HERE = new URL('.', import.meta.url);
const GLUE = 'bowery_wasm.js';
const WASM = 'bowery_wasm_bg.wasm';
const STYLE = 'bowery.css';

/** One import and one init per base, shared by every element. */
const modules = new Map();

function loadModule(base) {
  const key = base.href;
  if (!modules.has(key)) {
    modules.set(
      key,
      import(new URL(GLUE, base).href).then(async (glue) => {
        await glue.default({ module_or_path: new URL(WASM, base) });
        return glue;
      }),
    );
  }
  return modules.get(key);
}

/** mdBook's dark themes, or the system's dark preference. */
function prefersDark() {
  const html = document.documentElement.classList;
  if (html.contains('coal') || html.contains('navy') || html.contains('ayu')) return true;
  if (html.contains('light') || html.contains('rust')) return false;
  return window.matchMedia('(prefers-color-scheme: dark)').matches;
}

/** A button; `name` is its accessible name when the label is a symbol. */
function button(label, onClick, name) {
  const element = document.createElement('button');
  element.type = 'button';
  element.textContent = label;
  if (name) {
    element.setAttribute('aria-label', name);
    element.title = name;
  }
  element.addEventListener('click', onClick);
  return element;
}

class BoweryTableElement extends HTMLElement {
  static get observedAttributes() {
    return ['src'];
  }

  connectedCallback() {
    if (this._wired) return;
    this._wired = true;
    this._base = new URL(this.getAttribute('base') || HERE.href, document.baseURI);
    if (!this.hasAttribute('tabindex')) this.tabIndex = 0;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          observer.disconnect();
          this._start();
        }
      },
      { rootMargin: '100% 0px' },
    );
    observer.observe(this);
  }

  disconnectedCallback() {
    clearTimeout(this._timer);
    this._resize?.disconnect();
  }

  attributeChangedCallback(name, before, after) {
    if (name === 'src' && this._table && before !== after) this._reload();
  }

  async _reload() {
    clearTimeout(this._timer);
    this._table?.free?.();
    this._table = null;
    await this._start();
  }

  async _start() {
    if (location.protocol === 'file:') {
      this._fail('this table needs a served page');
      return;
    }
    let yaml;
    try {
      yaml = await this._config();
    } catch (error) {
      this._fail(`the config did not load: ${error.message}`);
      return;
    }
    let glue;
    try {
      glue = await loadModule(this._base);
    } catch (error) {
      this._fail('the table did not load');
      this._emit('bowery:error', { message: String(error) });
      return;
    }
    try {
      this._table = new glue.BoweryTable(yaml);
    } catch (error) {
      this._problems(String(error));
      return;
    }
    this._version = glue.BoweryTable.version();
    this._mount();
    this._render();
    this._emit('bowery:ready', { version: this._version });
    this._pace();
  }

  /** The inline YAML, or the file `src` names (same origin only). */
  async _config() {
    const src = this.getAttribute('src');
    if (src) {
      const url = new URL(src, document.baseURI);
      if (url.origin !== location.origin) throw new Error('src must be on this page\'s origin');
      const response = await fetch(url);
      if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
      return response.text();
    }
    const script = this.querySelector('script[type="application/yaml"]');
    if (!script) throw new Error('no <script type="application/yaml"> and no src');
    return script.textContent;
  }

  /** The fallback stays; a note goes under it. */
  _fail(message) {
    const note = document.createElement('p');
    note.className = 'bowery-note';
    note.textContent = message;
    this.append(note);
  }

  /** The fallback stays; the config's problems go under it. */
  _problems(json) {
    let lines;
    try {
      lines = JSON.parse(json).map((problem) => (problem.path ? `${problem.path}: ${problem.message}` : problem.message));
    } catch {
      lines = [json];
    }
    const pre = document.createElement('pre');
    pre.className = 'bowery-problems';
    pre.textContent = lines.join('\n');
    this.append(pre);
    this._emit('bowery:error', { problems: lines });
  }

  _mount() {
    const root = this.shadowRoot || this.attachShadow({ mode: 'open' });
    root.replaceChildren();
    // A custom element is inline until bowery.css arrives; block at once, so
    // the first frame is measured and drawn at the element's real width.
    const block = document.createElement('style');
    block.textContent = ':host { display: block; }';
    const style = document.createElement('link');
    style.rel = 'stylesheet';
    style.href = new URL(STYLE, this._base).href;
    this._stage = document.createElement('div');
    this._stage.className = 'stage';
    this._bar = document.createElement('div');
    this._bar.className = 'bar';
    this._status = document.createElement('div');
    this._status.className = 'status';
    this._status.setAttribute('aria-live', 'polite');
    root.append(block, style, this._stage, this._bar, this._status);

    this._width = this.clientWidth || 680;
    this._resize = new ResizeObserver((entries) => {
      const width = Math.round(entries[0].contentRect.width);
      if (width > 0 && width !== this._width) {
        this._width = width;
        this._draw();
      }
    });
    this._resize.observe(this);
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => this._render());
    new MutationObserver(() => this._render()).observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class'],
    });
    this.addEventListener('keydown', (event) => this._key(event));
  }

  _state() {
    return JSON.parse(this._table.state());
  }

  _draw() {
    const dark = prefersDark();
    this.toggleAttribute('data-dark', dark);
    // The SVG is bowery's own renderer's: every name and caption from the
    // config is XML-escaped there (tests/draw.rs draw__names_are_escaped),
    // and it holds no script.
    this._stage.innerHTML = this._table.svg(this._width, dark);
  }

  _render() {
    if (!this._table) return;
    const before = this._last;
    const state = this._state();
    this._last = state;
    this._draw();
    this._controls(state);
    this._status.textContent = state.caption;
    this._emit('bowery:moment', { moment: state.moment - 1, moments: state.moments });
    if (state.record === 'off' && before?.record !== 'off') this._emit('bowery:off-record', { since: state.since });
    if (state.hand_over && !before?.hand_over) this._emit('bowery:hand-over', { result: state.result });
  }

  /** The bar under the table, rebuilt for the state; focus stays put. */
  _controls(state) {
    const focused = this.shadowRoot.activeElement?.dataset?.control;
    const bar = this._bar;
    bar.replaceChildren();
    const add = (element, control) => {
      element.dataset.control = control;
      bar.append(element);
      return element;
    };
    if (state.controls.seek) {
      add(button('⏮', () => this._seek(0), 'first'), 'first');
      add(button('◀', () => this._seek(state.moment - 2), 'back'), 'back');
      add(button('▶', () => this._step(), 'next'), 'next');
      add(button('⏭', () => this._seek(state.moments - 1), 'last'), 'last');
      const readout = document.createElement('span');
      readout.className = 'readout';
      readout.textContent = `${state.moment} / ${state.moments}`;
      add(readout, 'readout');
    }
    if (state.controls.take_seat && !state.hand_over) {
      const select = document.createElement('select');
      select.setAttribute('aria-label', 'seat');
      for (const seat of state.seats.filter((seat) => seat.live)) {
        const option = document.createElement('option');
        option.value = seat.seat;
        option.textContent = seat.name;
        option.selected = seat.seat === state.reader;
        select.append(option);
      }
      add(select, 'seat');
      add(button('Take this seat', () => this._take(Number(select.value))), 'take');
    }
    if (state.controls.back_to_record) add(button('Back to the record', () => this._back()), 'back-to-record');
    // Empty unless the reader can act at this moment.
    if (state.legal.length > 0) this._actions(state, add);
    if (state.controls.next_hand) add(button('Next hand', () => this._next()), 'next-hand');
    if (state.controls.shuffle) add(button('Shuffle', () => this._shuffle()), 'shuffle');
    if (state.controls.view) {
      add(button(`👁 ${state.view}`, () => this._toggleView(state.view)), 'view');
    }
    if (state.controls.layout) {
      add(button(`▦ ${state.layout}`, () => this._toggleLayout(state.layout)), 'layout');
    }
    if (state.controls.odds) {
      const odds = add(button(`% ${state.odds ? 'on' : 'off'}`, () => this._toggleOdds(state.odds), 'odds'), 'odds');
      odds.setAttribute('aria-pressed', String(state.odds));
    }
    const again = focused && bar.querySelector(`[data-control="${focused}"]`);
    if (again) again.focus();
  }

  /** The reader's legal actions as buttons; a size box for bet or raise. */
  _actions(state, add) {
    for (const legal of state.legal) {
      if (legal.action === 'bet' || legal.action === 'raise') {
        const size = document.createElement('input');
        size.type = 'number';
        size.min = legal.min;
        size.max = legal.max;
        size.value = legal.min;
        size.setAttribute('aria-label', `${legal.action} to`);
        add(size, `${legal.action}-size`);
        add(
          button(legal.action === 'bet' ? 'Bet' : 'Raise to', () =>
            this._act({ action: legal.action, amount: Number(size.value) }),
          ),
          legal.action,
        );
      } else {
        const label = {
          fold: 'Fold',
          check: 'Check',
          call: `Call ${legal.amount?.toLocaleString('en-US') ?? ''}`.trim(),
          all_in: 'All in',
        }[legal.action];
        add(button(label, () => this._act({ action: legal.action })), legal.action);
      }
    }
  }

  _key(event) {
    const state = this._last;
    // Keys from the shadow DOM are retargeted to this element, so
    // `event.target` is always `this`; the path's first entry is the control
    // that had focus. Its keys are its own.
    if (!state?.controls.seek || event.composedPath()[0] !== this) return;
    const moves = {
      ArrowLeft: state.moment - 2,
      ArrowRight: state.moment,
      Home: 0,
      End: state.moments - 1,
    };
    if (event.key in moves) {
      event.preventDefault();
      this._seek(moves[event.key]);
    }
  }

  _seek(index) {
    clearTimeout(this._timer);
    this._table.seek(Math.max(0, index));
    this._render();
  }

  /** ▶: one moment on; on the record it stops at the reader's turn. */
  _step() {
    this._try(() => this._table.advance());
    this._pace();
  }

  _take(seat) {
    this._try(() => this._table.take_seat(seat));
  }

  _back() {
    clearTimeout(this._timer);
    this._table.back_to_record();
    this._render();
  }

  _act(action) {
    this._try(() => this._table.act(JSON.stringify(action)));
    this._pace();
  }

  _next() {
    this._try(() => this._table.next_hand());
    this._pace();
  }

  _shuffle() {
    clearTimeout(this._timer);
    this._try(() => this._table.shuffle((Math.random() * 0xffffffff) >>> 0));
    this._pace();
  }

  _toggleView(view) {
    this._table.set_view(view === 'all' ? 'seat' : 'all');
    this._render();
  }

  _toggleOdds(on) {
    this._table.set_odds(!on);
    this._render();
  }

  _toggleLayout(layout) {
    const order = ['ring', 'spotlight', 'ledger'];
    this._table.set_layout(order[(order.indexOf(layout) + 1) % order.length]);
    this._render();
  }

  _try(move) {
    try {
      move();
    } catch (error) {
      this._status.textContent = String(error);
      this._emit('bowery:error', { message: String(error) });
    }
    this._render();
  }

  /**
   * Off the record and in play, bots and deals go on by themselves, one
   * moment per `delay_ms`, while the latest moment is on screen and it is
   * not the reader's turn.
   */
  _pace() {
    clearTimeout(this._timer);
    const state = this._state();
    const live = state.mode === 'play' || state.record === 'off';
    if (!live || state.hand_over || state.moment !== state.moments) return;
    this._timer = setTimeout(() => {
      let answer;
      try {
        answer = this._table.advance();
      } catch (error) {
        this._emit('bowery:error', { message: String(error) });
        return;
      }
      this._render();
      if (answer === 'acted' || answer === 'dealt') this._pace();
    }, state.delay_ms);
  }

  _emit(name, detail) {
    this.dispatchEvent(new CustomEvent(name, { detail, bubbles: true, composed: true }));
  }
}

if (!customElements.get('bowery-table')) customElements.define('bowery-table', BoweryTableElement);

export { BoweryTableElement };

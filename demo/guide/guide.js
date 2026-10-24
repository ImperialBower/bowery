// The guide pages' own script: the theme switch, the event log, and a copy
// button on every code block. The tables come from bowery.js, as on the
// demo page.

document.getElementById('theme')?.addEventListener('change', (event) => {
  document.documentElement.className = event.target.value;
});

const log = document.getElementById('log');
if (log) {
  for (const name of ['bowery:ready', 'bowery:off-record', 'bowery:hand-over', 'bowery:error']) {
    document.addEventListener(name, (event) => {
      log.textContent += `${name} ${JSON.stringify(event.detail)}\n`;
    });
  }
}

// An abridged block (data-abridged) is not whole, so it gets no copy button:
// its page links the whole file instead.
for (const block of document.querySelectorAll('.code:not([data-abridged])')) {
  const copy = document.createElement('button');
  copy.type = 'button';
  copy.textContent = 'copy';
  copy.addEventListener('click', async () => {
    try {
      await navigator.clipboard.writeText(block.querySelector('pre').textContent);
      copy.textContent = 'copied';
    } catch {
      copy.textContent = 'select and copy';
    }
    setTimeout(() => (copy.textContent = 'copy'), 1500);
  });
  block.prepend(copy);
}

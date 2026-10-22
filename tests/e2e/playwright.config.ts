// The browser proof runs against `site/`, which `make e2e` assembles: the
// demo, plus the pages and configs under `site/e2e/`.
import { defineConfig, devices } from '@playwright/test';
import path from 'path';

export default defineConfig({
  testDir: '.',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [['github'], ['list']] : [['list']],
  use: {
    baseURL: 'http://localhost:8766',
    trace: 'retain-on-failure',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: 'python3 -m http.server -d site 8766',
    cwd: path.join(__dirname, '..', '..'),
    url: 'http://localhost:8766/index.html',
    reuseExistingServer: !process.env.CI,
    timeout: 10_000,
  },
});

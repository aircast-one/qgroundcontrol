import { type Browser, chromium, expect, type Page, test } from '@playwright/test';
import { openMenu } from '../e2e/fixtures';

const CDP = process.env.DESKTOP_CDP ?? 'http://127.0.0.1:9222';
const DEBUG_API = process.env.DESKTOP_DEBUG_API ?? 'http://127.0.0.1:28777';

test.describe.configure({ mode: 'serial' });

const shared: { browser: Browser | null; page: Page | null } = { browser: null, page: null };

const app = (): Page => {
  if (shared.page === null) throw new Error('The desktop app is not attached.');
  return shared.page;
};

test.beforeAll(async () => {
  shared.browser = await chromium.connectOverCDP(CDP);
  const page = shared.browser.contexts()[0]?.pages()[0] ?? null;
  if (page === null) throw new Error(`The desktop app at ${CDP} shows no page.`);
  shared.page = page;
});

test.afterAll(async () => {
  await shared.browser?.close();
});

test('the installed app opens on the welcome screen and goes on to the Fly screen', async () => {
  const welcome = app().getByRole('dialog').filter({ has: app().getByRole('heading', { name: 'Welcome to Aircast' }) });
  await welcome.getByRole('button', { name: 'Continue' }).click();
  await expect(welcome).toBeHidden();
  const menu = app().getByRole('button', { name: 'Menu', exact: true });
  const rail = app().getByRole('navigation').getByRole('button', { name: 'Settings', exact: true });
  const betaOffer = app().getByRole('dialog', { name: 'Help us debug this beta' }).getByRole('button', { name: 'No thanks' });
  await expect(menu.or(rail).or(betaOffer).first()).toBeVisible();
  if (await betaOffer.isVisible()) await betaOffer.click();
  await expect(menu.or(rail).first()).toBeVisible();
});

test('a vehicle reaches the page through the native core', async () => {
  const started = await fetch(`${DEBUG_API}/links/mocklink?autopilot=arducopter`);
  expect(started.ok).toBe(true);
  await expect(app().getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });
});

test('general settings show the version and its update channel', async () => {
  await openMenu(app(), 'Settings');
  await app().getByRole('tablist').getByRole('tab', { name: 'General' }).click();
  await expect(app().getByText('Aircast version', { exact: true })).toBeVisible();
  await expect(app().getByText(/ · (beta|stable) channel$/)).toBeVisible();
  await expect(app().getByRole('switch', { name: 'Check for updates automatically' })).toBeVisible();
});

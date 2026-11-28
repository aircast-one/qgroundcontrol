import { readFile, writeFile } from 'node:fs/promises';
import { type Browser, chromium, expect, type Page, test } from '@playwright/test';
import { actionOrder, mouseDragAction, openMenu } from '../e2e/fixtures';

const CDP = process.env.DESKTOP_CDP ?? 'http://127.0.0.1:9222';
const DEBUG_API = process.env.DESKTOP_DEBUG_API ?? 'http://127.0.0.1:28777';

test.describe.configure({ mode: 'serial' });

const shared: { browser: Browser | null; page: Page | null } = { browser: null, page: null };

const app = (): Page => {
  if (shared.page === null) throw new Error('The desktop app is not attached.');
  return shared.page;
};

const allPages = (browser: Browser): Page[] => browser.contexts().flatMap((context) => context.pages());

const appPage = (browser: Browser): Page | null => allPages(browser).find((page) => /^https?:\/\/tauri\.localhost|^tauri:/.test(page.url())) ?? null;

test.beforeAll(async () => {
  const browser = await chromium.connectOverCDP(CDP);
  shared.browser = browser;
  await expect.poll(() => appPage(browser)?.url() ?? allPages(browser).map((page) => page.url()).join(' | '), { timeout: 60_000 }).toMatch(/tauri/);
  shared.page = appPage(browser);
});

test.afterEach(async ({}, testInfo) => {
  if (testInfo.status === testInfo.expectedStatus || shared.browser === null) return;
  const pages = allPages(shared.browser);
  const described = await Promise.all(
    pages.map(async (page) => `${page.url()} closed=${page.isClosed()} ready=${page.isClosed() ? '-' : await page.evaluate(() => document.readyState).catch((error: Error) => error.message)}`),
  );
  const text = shared.page === null || shared.page.isClosed() ? '' : await shared.page.evaluate(() => document.body.innerText.slice(0, 4000)).catch((error: Error) => error.message);
  const log = await fetch(`${DEBUG_API}/bridge/get?path=view.appLog`).then((response) => response.text()).catch((error: Error) => error.message);
  await writeFile(testInfo.outputPath('desktop-state.txt'), [`pages:\n${described.join('\n')}`, `text:\n${text}`, `appLog:\n${log.slice(-20_000)}`].join('\n\n'));
  if (shared.page !== null && !shared.page.isClosed()) await shared.page.screenshot({ path: testInfo.outputPath('desktop.png') }).catch(() => undefined);
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

test('the menu keys work while the page has focus: Ctrl+2 opens Plan and Ctrl+, opens Settings', async () => {
  await app().addLocatorHandler(app().getByRole('dialog').filter({ hasText: 'Aircraft connected' }), (prompt) => prompt.getByRole('button', { name: 'Keep my route' }).click());
  await app().keyboard.press('Control+Digit2');
  await expect(app().getByText('Click the map to add a waypoint')).toBeVisible();
  await app().keyboard.press('Control+Comma');
  await expect(app().getByRole('tablist').getByRole('tab', { name: 'General' })).toBeVisible();
});

const PHOTO = 2000;
const MOUNT = 205;

test('an action on a waypoint is dragged into place with the mouse in the webview, and the mission follows', async ({}, testInfo) => {
  await app().keyboard.press('Control+Digit2');
  await expect(app().getByText('Click the map to add a waypoint')).toBeVisible();
  const map = (await app().getByRole('region', { name: 'Map' }).boundingBox())!;
  await app().mouse.click(map.x + map.width * 0.6, map.y + map.height * 0.5);
  await expect(app().getByRole('button', { name: 'Delete waypoint 1' })).toBeVisible({ timeout: 30_000 });
  await ['Tilt camera', 'Photo'].reduce(
    (done, action) =>
      done.then(async () => {
        await app().getByRole('button', { name: 'Add action', exact: true }).click();
        await app().getByRole('menuitem', { name: action }).click();
        await app().getByRole('dialog').getByRole('button', { name: 'Done' }).click();
      }),
    Promise.resolve(),
  );
  const order = actionOrder(app());
  await expect(order).toHaveText([/^Gimbal/, /^Take photo/]);
  await mouseDragAction(app(), 'Take photo', order.nth(0));
  await expect(order, 'the photo now comes before the camera aim').toHaveText([/^Take photo/, /^Gimbal/], { timeout: 15_000 });
  const file = testInfo.outputPath('dragged.plan');
  await fetch(`${DEBUG_API}/bridge/invoke?path=core.plan.save&args=${encodeURIComponent(JSON.stringify([file]))}`);
  const saved = async (): Promise<readonly number[]> => {
    const plan = JSON.parse(await readFile(file, 'utf8').catch(() => '{}')) as { mission?: { items?: { command?: number }[] } };
    return (plan.mission?.items ?? []).map((item) => item.command ?? -1).filter((command) => command === PHOTO || command === MOUNT);
  };
  await expect.poll(saved, { message: 'the saved mission takes the photo before it aims the camera' }).toEqual([PHOTO, MOUNT]);
  await app().keyboard.press('Control+Comma');
  await expect(app().getByRole('tablist').getByRole('tab', { name: 'General' })).toBeVisible();
});

test('a TCP address that does not answer says why on the page, keeps what was typed and leaves no link behind', async () => {
  await app().getByRole('tablist').getByRole('tab', { name: 'Transmission' }).click();
  await app().getByRole('button', { name: /^Links/ }).first().click();
  await app().getByRole('button', { name: 'Add a link' }).last().click();
  await app().getByRole('button', { name: /^Connect to an address \(TCP\)/ }).click();
  await app().getByRole('textbox', { name: 'Address', exact: true }).fill('tcp://127.0.0.1:9');
  await app().getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(app().getByRole('textbox', { name: 'Address', exact: true })).toHaveValue('127.0.0.1');
  await expect(app().getByRole('textbox', { name: 'Port', exact: true })).toHaveValue('9');
  await expect(app().getByRole('alert')).toBeVisible({ timeout: 30_000 });
  await expect(app().getByRole('button', { name: 'Connect', exact: true })).toBeEnabled();
  const links = await fetch(`${DEBUG_API}/links`).then((response) => response.text());
  expect(links, 'the failed link was removed, so trying again does not leave a duplicate').not.toContain('127.0.0.1');
});

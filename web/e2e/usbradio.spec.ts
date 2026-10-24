import type { Page } from '@playwright/test';
import { expect, openApp, test } from './fixtures';
import { lastAnswer, plugInR2d2Radio, setRadioChannels, setRadioStreaming } from './r2d2';
import { type Sitl, startSitl } from './sitl';

const ROLL = 0;
const THROTTLE = 2;
const DROP = 10;

const sitl: { running: Sitl | null } = { running: null };

test.describe.configure({ mode: 'serial' });

test.beforeAll(async () => {
  test.setTimeout(120_000);
  const running = startSitl();
  sitl.running = running;
  await expect.poll(() => running.heard.heartbeat, { timeout: 90_000 }).toBe(true);
});

test.afterAll(() => sitl.running?.stop());

const running = (): Sitl => {
  if (sitl.running === null) throw new Error('The SITL did not start');
  return sitl.running;
};

const channel = (index: number) => () => running().heard.rc[index] ?? 0;

const connectSitl = async (page: Page) => {
  await page.getByRole('button', { name: /How to connect/ }).click();
  await page.getByRole('button', { name: 'Add link…' }).click();
  await page.getByRole('button', { name: /^Network \(WebSocket\)/ }).click();
  await page.getByRole('textbox', { name: 'WebSocket address' }).fill(running().ws);
  await page.getByRole('textbox', { name: 'WebSocket address' }).locator('xpath=following::button[normalize-space()="Connect"][1]').click();
  await expect(page.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 120_000 });
};

const RADIO_FOUND = 'An EdgeTX radio is streaming its sticks on USB. Fly with it?';
const DETECTED_RADIO = 'EdgeTX radio (webserial-0)';

const openRadioSettings = async (page: Page) => {
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('tablist').getByRole('tab', { name: 'Control' }).click();
  await expect(page.getByText('Radio on USB (EdgeTX script)')).toBeVisible();
};

const radioDevice = (page: Page) => page.getByText('Radio on USB (EdgeTX script)').locator('xpath=following::button[@aria-haspopup="menu"][1]');

const indicator = (page: Page, text: string) => page.getByText(text, { exact: true });

test('a radio streaming the script is found by what it sends, offered once, and flies ArduPilot on its own channel numbers', async ({ page }) => {
  test.setTimeout(300_000);
  await page.context().route((url) => !['localhost', '127.0.0.1'].includes(url.hostname), (route) => route.abort());
  await page.addLocatorHandler(page.getByText('Configuration tasks remain before this vehicle is ready to fly.', { exact: false }), () => page.getByRole('button', { name: 'Later', exact: true }).click());
  await plugInR2d2Radio(page);
  await openApp(page);

  await expect(page.getByText(RADIO_FOUND), 'the auto-connect link that opened the radio as an ArduPilot board hears its lines instead of MAVLink').toBeVisible({ timeout: 30_000 });
  await page.getByRole('button', { name: 'Use', exact: true }).click();

  await connectSitl(page);
  await page.getByRole('button', { name: 'All status' }).click();
  await expect(indicator(page, 'Radio · live')).toBeVisible({ timeout: 20_000 });
  await page.keyboard.press('Escape');
  await expect(indicator(page, 'Radio · live')).toBeHidden();
  await expect.poll(() => lastAnswer(page), { message: 'the radio is told a copter flies on it and which channel does what', timeout: 10_000 }).toMatch(/^@F,\d+,C,1,2,3,4$/);
  await setRadioChannels(page, { 1: 1024, 3: -1024, 11: 1024 });
  await expect.poll(channel(ROLL), { message: 'roll right', timeout: 20_000 }).toBeGreaterThan(1900);
  await expect.poll(channel(THROTTLE), { message: 'throttle low' }).toBeLessThan(1050);
  await expect.poll(channel(DROP), { message: 'the payload switch on channel eleven' }).toBeGreaterThan(1900);
  await setRadioChannels(page, { 1: -1024, 11: -1024 });
  await expect.poll(channel(ROLL), { message: 'roll left' }).toBeLessThan(1100);
  await expect.poll(channel(DROP), { message: 'payload switch back' }).toBeLessThan(1100);

  await openRadioSettings(page);
  await expect(radioDevice(page), 'the same 0483:5740 an ArduPilot board shows is named by what it sends').toHaveAccessibleName(DETECTED_RADIO);
  await radioDevice(page).click();
  await expect(page.getByRole('menuitem', { name: 'Allow a USB port…' })).toBeVisible();
  await page.getByRole('menuitem', { name: 'Off', exact: true }).click();
  await expect.poll(channel(ROLL), { message: 'switched off, roll goes back to the SITL radio', timeout: 10_000 }).toBeGreaterThan(1400);
  await expect.poll(channel(ROLL)).toBeLessThan(1600);
  await expect(page.getByText(RADIO_FOUND), 'Not now or Off is not asked again for the same port').toBeHidden();

  await radioDevice(page).click();
  await page.getByRole('menuitem', { name: DETECTED_RADIO }).click();
  await page.getByRole('button', { name: 'Close settings' }).click();
  await setRadioChannels(page, { 1: 1024 });
  await expect.poll(channel(ROLL), { message: 'picked again', timeout: 20_000 }).toBeGreaterThan(1900);
  await setRadioStreaming(page, false);
  await expect.poll(channel(ROLL), { message: 'a radio that stops streaming hands the channels back', timeout: 10_000 }).toBeLessThan(1600);
  await page.getByRole('button', { name: 'All status' }).click();
  await expect(indicator(page, 'Radio · stopped'), 'still chosen, but its sticks are handed back').toBeVisible({ timeout: 10_000 });
});

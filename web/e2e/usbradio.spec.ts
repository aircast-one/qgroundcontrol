import type { Page } from '@playwright/test';
import { expect, openApp, test } from './fixtures';
import { plugInR2d2Radio, RADIO_PORT_LABEL, setRadioChannels, setRadioStreaming } from './r2d2';
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
  await page.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(page.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 120_000 });
};

const openLinks = async (page: Page) => {
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('tablist').getByRole('tab', { name: 'Transmission' }).click();
  await page.getByRole('button', { name: /^Links/ }).click();
  await page.getByRole('button', { name: 'Advanced', exact: true }).click();
  await expect(page.getByText('Radio on USB (EdgeTX script)')).toBeVisible();
};

const radioDevice = (page: Page) => page.getByText('Radio on USB (EdgeTX script)').locator('xpath=following::button[@aria-haspopup="menu"][1]');

const chooseDevice = async (page: Page, name: string | RegExp) => {
  await radioDevice(page).click();
  await page.getByRole('menuitem', { name }).click();
};

test('a radio streaming the R2D2 script over USB flies ArduPilot on its own channel numbers and hands them back when switched off', async ({ page }) => {
  test.setTimeout(300_000);
  await page.context().route((url) => !['localhost', '127.0.0.1'].includes(url.hostname), (route) => route.abort());
  await plugInR2d2Radio(page);
  await openApp(page);
  await connectSitl(page);
  await openLinks(page);
  await expect(page.getByText('ArduPilot ChibiOS on webserial-0 (AutoConnect)')).toBeVisible({ timeout: 20_000 });

  await chooseDevice(page, RADIO_PORT_LABEL);
  await expect(radioDevice(page)).toHaveAccessibleName(RADIO_PORT_LABEL);
  await expect(page.getByText('ArduPilot ChibiOS on webserial-0 (AutoConnect)'), 'an EdgeTX radio reads as an ArduPilot board, so auto-connect had taken its port').toBeHidden({ timeout: 20_000 });

  await setRadioChannels(page, { 1: 1024, 3: -1024, 11: 1024 });
  await expect.poll(channel(ROLL), { message: 'roll right', timeout: 20_000 }).toBeGreaterThan(1900);
  await expect.poll(channel(THROTTLE), { message: 'throttle low' }).toBeLessThan(1050);
  await expect.poll(channel(DROP), { message: 'the payload switch on channel eleven' }).toBeGreaterThan(1900);
  await setRadioChannels(page, { 1: -1024, 11: -1024 });
  await expect.poll(channel(ROLL), { message: 'roll left' }).toBeLessThan(1100);
  await expect.poll(channel(DROP), { message: 'payload switch back' }).toBeLessThan(1100);

  await chooseDevice(page, 'Off');
  await expect.poll(channel(ROLL), { message: 'switched off, roll goes back to the SITL radio', timeout: 10_000 }).toBeGreaterThan(1400);
  await expect.poll(channel(ROLL)).toBeLessThan(1600);

  await chooseDevice(page, RADIO_PORT_LABEL);
  await setRadioChannels(page, { 1: 1024 });
  await expect.poll(channel(ROLL), { message: 'picked again', timeout: 20_000 }).toBeGreaterThan(1900);
  await setRadioStreaming(page, false);
  await expect.poll(channel(ROLL), { message: 'a radio that stops streaming hands the channels back', timeout: 10_000 }).toBeLessThan(1600);
});

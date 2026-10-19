import type { Page } from '@playwright/test';
import { expect, openAircraftSetup, openApp, test } from './fixtures';
import { moveSticks, plugInRadio, type Sticks } from './radio';
import { type Sitl, startSitl } from './sitl';

const CENTRE: Sticks = { roll: 0, pitch: 0, throttle: 0, yaw: 0 };
const IDLE: Sticks = { ...CENTRE, throttle: -1 };

const prompts = (rollRight: number): readonly (readonly [RegExp, Sticks])[] => [
  [/Throttle stick all the way up/, { ...CENTRE, throttle: 1 }],
  [/Throttle stick all the way down/, IDLE],
  [/Yaw stick all the way to the right/, { ...IDLE, yaw: 1 }],
  [/Yaw stick all the way to the left/, { ...IDLE, yaw: -1 }],
  [/Roll stick all the way to the right/, { ...IDLE, roll: rollRight }],
  [/Roll stick all the way to the left/, { ...IDLE, roll: -rollRight }],
  [/Pitch stick all the way up/, { ...IDLE, pitch: 1 }],
  [/Pitch stick all the way down/, { ...IDLE, pitch: -1 }],
];

const ROLL = 0;
const PITCH = 1;
const THROTTLE = 2;
const YAW = 3;

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

const calibration = (page: Page) =>
  page.evaluate(async (module) => (await ((await import(module)) as { Qgc: { get: (path: string) => Promise<{ calibration?: unknown }> } }).Qgc.get('view.joystick')).calibration ?? null, '/src/bridge/Qgc.ts');

const connectSitl = async (page: Page) => {
  await page.getByRole('button', { name: /How to connect/ }).click();
  await page.getByRole('button', { name: 'Add link…' }).click();
  await page.getByRole('button', { name: /^Network \(WebSocket\)/ }).click();
  await page.getByRole('textbox', { name: 'WebSocket address' }).fill(running().ws);
  await page.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Open Setup', exact: true })).toBeVisible({ timeout: 120_000 });
};

const captureSticks = async (page: Page, rollRight = 1) => {
  await page.getByRole('button', { name: /^Joystick.*axes/ }).click();
  await expect(page.getByText(/^Stick mode/)).toBeVisible();
  await expect(page.getByRole('checkbox', { name: 'Enable' })).toBeDisabled();
  await expect(page.getByText('Calibrate before enabling')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Cancel', exact: true })).toBeHidden();
  await page.getByRole('button', { name: 'Calibrate', exact: true }).click();
  await expect(page.getByText('Step 1 of 10')).toBeVisible();
  await expect(page.getByText('Center all sticks as shown in diagram.')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Advanced settings' })).toBeHidden();
  await expect(page.getByText('Axis monitor')).toBeHidden();
  await moveSticks(page, CENTRE);
  await page.getByRole('button', { name: 'Next', exact: true }).click();
  await prompts(rollRight).reduce(async (previous, [prompt, sticks]) => {
    await previous;
    await expect(page.getByText(prompt)).toBeVisible();
    await moveSticks(page, sticks);
    await expect(page.getByText(prompt)).toBeHidden({ timeout: 45_000 });
  }, Promise.resolve());
  await expect(page.getByText(/All sticks are captured/)).toBeVisible();
  await expect(page.getByText('Step 10 of 10')).toBeVisible();
  await expect(page.getByText(/^Save keeps this calibration in this browser for /)).toBeVisible();
};

const calibrate = async (page: Page) => {
  await captureSticks(page);
  await expect(page.getByText(/moved the opposite way/)).toBeHidden();
  await moveSticks(page, { ...IDLE, roll: 1 });
  await expect(page.getByRole('progressbar', { name: 'Roll stick' })).toHaveAttribute('aria-valuenow', /^(9\d|100)$/);
  await moveSticks(page, IDLE);
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await page.getByRole('button', { name: 'Yes', exact: true }).click();
  await expect(page.getByRole('checkbox', { name: 'Enable' })).toBeChecked();
  await expect(page.getByText(/^Calibrated · saved in this browser for /)).toBeVisible();
  await expect(page.getByText('Sending your sticks to the aircraft')).toBeVisible();
};

test('a roll stick pushed the wrong way is flagged before saving, and leaving the page drops the unsaved calibration', async ({ page }) => {
  test.setTimeout(300_000);
  await page.context().route((url) => !['localhost', '127.0.0.1'].includes(url.hostname), (route) => route.abort());
  await plugInRadio(page);
  await openApp(page);
  await connectSitl(page);
  await page.getByRole('button', { name: 'Open Setup', exact: true }).click();
  await captureSticks(page, -1);
  await expect(page.getByText(/^Roll moved the opposite way to yaw/)).toBeVisible();
  await page.getByRole('button', { name: 'Close settings' }).click();
  await expect.poll(() => calibration(page)).toBeNull();
  await openAircraftSetup(page);
  await page.getByRole('button', { name: /^Joystick.*axes/ }).click();
  await expect(page.getByText('Calibrate before enabling')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Calibrate', exact: true })).toBeVisible();
});

test('a radio calibrated in the browser moves each ArduPilot channel the way its stick points', async ({ page }) => {
  test.setTimeout(300_000);
  await page.context().route((url) => !['localhost', '127.0.0.1'].includes(url.hostname), (route) => route.abort());
  await plugInRadio(page);
  await openApp(page);
  await connectSitl(page);
  await page.getByRole('button', { name: 'Open Setup', exact: true }).click();
  await calibrate(page);

  await moveSticks(page, IDLE);
  await expect.poll(channel(THROTTLE), { timeout: 20_000 }).toBeLessThan(1050);
  await expect.poll(channel(ROLL)).toBeGreaterThan(1450);
  await expect.poll(channel(ROLL)).toBeLessThan(1550);

  await moveSticks(page, { ...IDLE, roll: 1 });
  await expect.poll(channel(ROLL), { message: 'roll right' }).toBeGreaterThan(1900);
  await moveSticks(page, { ...IDLE, roll: -1 });
  await expect.poll(channel(ROLL), { message: 'roll left' }).toBeLessThan(1100);

  await moveSticks(page, { ...IDLE, pitch: 1 });
  await expect.poll(channel(PITCH), { message: 'pitch forward lowers the pitch channel on ArduPilot' }).toBeLessThan(1100);
  await moveSticks(page, { ...IDLE, pitch: -1 });
  await expect.poll(channel(PITCH), { message: 'pitch back' }).toBeGreaterThan(1900);

  await moveSticks(page, { ...IDLE, yaw: 1 });
  await expect.poll(channel(YAW), { message: 'yaw right' }).toBeGreaterThan(1900);
  await moveSticks(page, { ...IDLE, yaw: -1 });
  await expect.poll(channel(YAW), { message: 'yaw left' }).toBeLessThan(1100);

  await moveSticks(page, { ...CENTRE, throttle: 1 });
  await expect.poll(channel(THROTTLE), { message: 'throttle up' }).toBeGreaterThan(1900);
});

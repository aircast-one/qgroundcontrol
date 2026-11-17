import { type Locator, type Page } from '@playwright/test';
import { connectMock, expect, openAircraftSetup, openApp, openMenu, openPlanAtVehicle, tapPlanMission, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

const UNNAMED_CONTROL = /^\s*- (textbox|slider|switch|checkbox|combobox|spinbutton|radio|radiogroup|dialog)(:|\s*\[|$)/;

const unnamedControls = async (page: Page): Promise<readonly string[]> =>
  (await page.locator('body').ariaSnapshot()).split('\n').filter((line) => UNNAMED_CONTROL.test(line)).map((line) => line.trim());

const valueOnlyNames = async (page: Page): Promise<readonly string[]> => {
  const choices = await page.locator('button[aria-haspopup="menu"]:visible').evaluateAll((buttons) => buttons.map((it) => it.getAttribute('aria-label') ?? ''));
  const helps = await page.getByRole('button', { name: /^(Help|Hide help)$/ }).count();
  return [...choices.filter((name) => !name.includes(', ')), ...(helps > 0 ? [`${helps} help buttons named only "Help"`] : [])];
};

test('every form control on Fly, Plan and the settings tabs has an accessible name', async ({ vehicle }) => {
  expect(await unnamedControls(vehicle)).toEqual([]);
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  expect(await unnamedControls(vehicle)).toEqual([]);
  await openMenu(vehicle, 'Settings');
  const tabs = await vehicle.getByRole('tablist').getByRole('tab').allTextContents();
  await tabs.reduce(
    (done, tab) =>
      done.then(async () => {
        await vehicle.getByRole('tablist').getByRole('tab', { name: tab }).click();
        expect(await unnamedControls(vehicle), tab).toEqual([]);
        expect(await valueOnlyNames(vehicle), tab).toEqual([]);
      }),
    Promise.resolve(),
  );
});

const ANALYZE_PAGES = ['Flight logs', 'Geotag images', 'Vibration', 'MAVLink inspector', 'Console', 'Messages', 'Firmware'] as const;

test('every form control on the Analyze pages has an accessible name', async ({ vehicle }) => {
  await openMenu(vehicle, 'Analyze');
  await ANALYZE_PAGES.reduce(
    (done, page) =>
      done.then(async () => {
        await vehicle.getByRole('button', { name: new RegExp(`^${page}`) }).click();
        await expect(vehicle.getByText(page, { exact: true }).first()).toBeVisible();
        expect(await unnamedControls(vehicle), page).toEqual([]);
      }),
    Promise.resolve(),
  );
});

const SETUP_PAGES = ['Sensors', 'Radio', 'Flight modes', 'Power', 'Motors', 'Safety', 'Tuning', 'Camera', 'Parameters'] as const;

test('every form control on the PX4 setup pages has an accessible name', async ({ vehicle }) => {
  await openAircraftSetup(vehicle);
  await SETUP_PAGES.reduce(
    (done, page) =>
      done.then(async () => {
        const entry = vehicle.getByRole('button', { name: new RegExp(`^${page}( |$)`) }).first();
        if (!(await entry.isVisible())) return;
        await entry.click();
        await expect(vehicle.getByText(page, { exact: true }).first()).toBeVisible();
        expect(await unnamedControls(vehicle), page).toEqual([]);
      }),
    Promise.resolve(),
  );
});

const APM_PAGES = ['Failsafes', 'Flight modes', 'Flight safety', 'Frame', 'Gimbal', 'Joystick', 'Logging', 'Motors', 'Power', 'Radio', 'Scripting', 'Sensors', 'Servo outputs', 'Tuning'] as const;

test('every form control on the ArduPilot setup pages has an accessible name', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await APM_PAGES.reduce(
    (done, page) =>
      done.then(async () => {
        await app.getByRole('button', { name: new RegExp(`^${page} `) }).first().click();
        await expect(app.getByText(page, { exact: true }).nth(1)).toBeVisible();
        expect(await unnamedControls(app), page).toEqual([]);
      }),
    Promise.resolve(),
  );
});

const tooltipOn = async (page: Page, target: Locator, text: string | RegExp) =>
  expect(async () => {
    await page.mouse.move(0, 0);
    await target.hover();
    await expect(page.getByRole('tooltip')).toHaveText(text, { timeout: 2_000 });
  }).toPass({ timeout: 30_000 });

test('icon buttons and status-strip glyphs say what they are on hover', async ({ vehicle }) => {
  await tooltipOn(vehicle, vehicle.getByRole('button', { name: /^\d+ sats$/ }), /^\d+ sats$/);
  await tooltipOn(vehicle, vehicle.getByRole('button', { name: /RC$/ }).first(), /RC$/);
  await openMenu(vehicle, 'Plan');
  await tooltipOn(vehicle, vehicle.getByRole('button', { name: 'Plan menu' }), 'Plan menu');
});

test('the Fly menu sheet and the take-off press are named', async ({ vehicle }) => {
  await expect(vehicle.getByRole('button', { name: /^Slide to take off/ })).toHaveAttribute('aria-label', /^Slide to take off/);
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  await expect(vehicle.getByRole('dialog', { name: 'Menu' })).toBeVisible();
  expect(await unnamedControls(vehicle)).toEqual([]);
});

test('the welcome names each choice by its question', async ({ page }) => {
  await openApp(page, { firstRun: false });
  const welcome = page.getByRole('dialog').filter({ has: page.getByRole('heading', { name: 'Welcome to Aircast' }) });
  await expect(welcome.getByRole('button', { name: /^Vehicle type, / })).toBeVisible();
  expect(await valueOnlyNames(page)).toEqual([]);
});

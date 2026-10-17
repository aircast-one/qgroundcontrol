import { type Page } from '@playwright/test';
import { connectMock, expect, openAircraftSetup, openMenu, openPlanAtVehicle, tapPlanMission, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

const UNNAMED_CONTROL = /^\s*- (textbox|slider|switch|checkbox|combobox|spinbutton|radio)(:|\s*\[|$)/;

const unnamedControls = async (page: Page): Promise<readonly string[]> =>
  (await page.locator('body').ariaSnapshot()).split('\n').filter((line) => UNNAMED_CONTROL.test(line)).map((line) => line.trim());

test('every form control on Fly, Plan and the settings tabs has an accessible name', async ({ vehicle }) => {
  expect(await unnamedControls(vehicle)).toEqual([]);
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  expect(await unnamedControls(vehicle)).toEqual([]);
  await vehicle.getByRole('button', { name: 'Settings', exact: true }).click();
  const tabs = await vehicle.getByRole('tablist').getByRole('tab').allTextContents();
  await tabs.reduce(
    (done, tab) =>
      done.then(async () => {
        await vehicle.getByRole('tablist').getByRole('tab', { name: tab }).click();
        expect(await unnamedControls(vehicle), tab).toEqual([]);
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

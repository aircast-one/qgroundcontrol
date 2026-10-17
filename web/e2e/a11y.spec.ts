import { type Page } from '@playwright/test';
import { expect, openPlanAtVehicle, tapPlanMission, test } from './fixtures';

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

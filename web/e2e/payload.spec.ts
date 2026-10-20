import type { Page } from '@playwright/test';
import { connectMock, expect, flightMode, itemCount, openAircraftSetup, openPlanAtVehicle, slideAcross, takeOff, tapPlanMission, test, vehicleStatus } from './fixtures';

test.describe.configure({ timeout: 180_000 });

const STAYS_OPEN = /The gripper stays open after the first drop, so the second drop releases nothing/;

test('a mission whose second drop would not fall says why and turns on auto-close in one tap', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  const drop = vehicle.getByRole('button', { name: 'Drop', exact: true });
  await drop.click();
  await drop.click();
  await vehicle.getByRole('button', { name: 'Upload', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: 'Upload', exact: true })).toBeHidden({ timeout: 30_000 });
  await vehicle.getByRole('button', { name: 'Fly', exact: true }).last().click();

  const status = await vehicleStatus(vehicle);
  await expect(status.getByText(STAYS_OPEN)).toBeVisible({ timeout: 30_000 });
  await vehicle.keyboard.press('Escape');
  await expect(vehicle.getByRole('button', { name: 'Slide to start mission' })).toBeHidden();

  await vehicle.getByRole('button', { name: 'More', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Payload').click();
  const panel = vehicle.getByRole('dialog').filter({ hasText: STAYS_OPEN });
  await expect(panel.getByRole('button', { name: 'Turn on auto-close' })).toBeVisible();
  await vehicle.keyboard.press('Escape');
  await expect(panel).toBeHidden();

  const sheet = (await vehicleStatus(vehicle)).filter({ hasText: STAYS_OPEN });
  await sheet.getByRole('button', { name: 'Turn on auto-close' }).click();
  await expect(vehicle.getByText(STAYS_OPEN)).toBeHidden({ timeout: 30_000 });
  await vehicle.keyboard.press('Escape');
  await expect(vehicle.getByText('Start mission', { exact: true })).toBeVisible({ timeout: 30_000 });
});

test('a drop after the last waypoint falls and ends the mission', async ({ app: vehicle }) => {
  test.setTimeout(300_000);
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await vehicle.getByRole('button', { name: 'Drop', exact: true }).click();
  await vehicle.getByRole('button', { name: 'Upload', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: 'Upload', exact: true })).toBeHidden({ timeout: 30_000 });
  await vehicle.getByRole('button', { name: 'Fly', exact: true }).last().click();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to start mission' }));
  const progress = vehicle.getByText(/^To waypoint \d+ of \d+/);
  await expect(progress).toBeVisible({ timeout: 30_000 });
  await expect(progress).toBeHidden({ timeout: 240_000 });
  await vehicle.getByRole('button', { name: 'More', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Payload').click();
  await slideAcross(vehicle, vehicle.getByRole('dialog').getByRole('button', { name: 'Slide to drop' }));
  await expect(vehicle.getByText('The gripper is still open from the last drop, so nothing would fall. Grab first, or turn on auto-close.')).toBeVisible();
});

test('a second drop while the gripper is still open is refused rather than counted', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await vehicle.getByRole('button', { name: 'More', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Payload').click();
  const sheet = vehicle.getByRole('dialog');
  await slideAcross(vehicle, sheet.getByRole('button', { name: 'Slide to drop' }));
  await expect(vehicle.getByText(/^Dropped \d+ s ago$/)).toBeVisible({ timeout: 30_000 });
  await slideAcross(vehicle, sheet.getByRole('button', { name: 'Slide to drop' }));
  await expect(vehicle.getByText('The gripper is still open from the last drop, so nothing would fall. Grab first, or turn on auto-close.')).toBeVisible();
  await sheet.getByRole('button', { name: 'Grab', exact: true }).click();
  await expect(vehicle.getByText(/^Grabbed \d+ s ago$/)).toBeVisible();
  await slideAcross(vehicle, sheet.getByRole('button', { name: 'Slide to drop' }));
  await expect(vehicle.getByText(/^Dropped \d+ s ago$/)).toBeVisible({ timeout: 30_000 });
});

test('a waypoint takes a drop from its actions and gives it back', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await expect.poll(() => itemCount(vehicle)).toBeGreaterThanOrEqual(5);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await vehicle.getByRole('button', { name: 'Waypoint 3' }).first().click();
  const before = await itemCount(vehicle);
  await vehicle.getByRole('button', { name: 'Add', exact: true }).click();
  await vehicle.getByRole('menuitem', { name: 'Drop payload' }).click();
  await expect.poll(() => itemCount(vehicle), { timeout: 30_000 }).toBe(before + 1);
  await expect(vehicle.getByRole('button', { name: 'Remove Drop payload' })).toBeVisible();
  await vehicle.getByRole('button', { name: 'Remove Drop payload' }).click();
  await expect.poll(() => itemCount(vehicle), { timeout: 30_000 }).toBe(before);
  await expect(vehicle.getByRole('button', { name: 'Add', exact: true })).toBeVisible();
});

test('a pilot flying by hand drops from the deck, and a drop that cannot fall says why', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await takeOff(vehicle, '10 m');
  await expect(vehicle.getByRole('button', { name: 'Slide to land' })).toBeVisible({ timeout: 60_000 });
  await vehicle.getByRole('button', { name: 'Change flight mode', exact: true }).click();
  await vehicle.getByRole('menu').getByRole('menuitem', { name: 'Loiter' }).click();
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Loiter/);
  const drop = vehicle.getByRole('button', { name: 'Slide to drop' });
  await slideAcross(vehicle, drop);
  await expect(vehicle.getByText('Payload dropped')).toBeVisible({ timeout: 30_000 });
  await drop.click();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to drop' }));
  await expect(vehicle.getByText('The gripper is still open from the last drop, so nothing would fall. Grab first, or turn on auto-close.')).toBeVisible({ timeout: 30_000 });
  expect(await vehicle.getByText('Sent · Drop').count(), 'a refused drop shows only the refusal').toBe(0);
  await expect(vehicle.getByRole('status').filter({ hasText: 'Payload dropped' }), 'the refusal is the newest payload result, so the earlier drop leaves the screen').toBeHidden({ timeout: 1_000 });
});

const BAY_OUTPUTS = [9, 10, 11, 12] as const;
const GRIP_RELEASE_PWM = '1100';

const fourServoBays = async (page: Page) => {
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByPlaceholder('Search settings').fill('bay');
  const bays = page.getByRole('textbox', { name: /^How many payload bays/ });
  await bays.fill('4');
  await bays.press('Enter');
  await page.getByRole('radio', { name: 'A servo per bay' }).click();
  await expect(page.getByText('Bay 4 output', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Close settings' }).click();
};

const servoPwm = (page: Page, output: number) => page.getByRole('progressbar', { name: `Servo ${output} position` }).locator('..');

const expectBaysReleased = async (page: Page) => {
  await openAircraftSetup(page);
  await page.getByRole('button', { name: /^Servo outputs/ }).click();
  await BAY_OUTPUTS.reduce(async (previous, output) => {
    await previous;
    await expect(servoPwm(page, output)).toContainText(GRIP_RELEASE_PWM, { timeout: 15_000 });
  }, Promise.resolve());
};

test('four bays on their own servos drop one after another by hand, each from its own output', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await fourServoBays(vehicle);
  await takeOff(vehicle, '10 m');
  await expect(vehicle.getByRole('button', { name: 'Slide to land' })).toBeVisible({ timeout: 60_000 });
  await vehicle.getByRole('button', { name: 'Change flight mode', exact: true }).click();
  await vehicle.getByRole('menu').getByRole('menuitem', { name: 'Loiter' }).click();
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Loiter/);
  await vehicle.getByRole('button', { name: 'More', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Payload').click();
  const sheet = vehicle.getByRole('dialog');
  await expect(sheet.getByText('Bay 2 · output 10', { exact: true })).toBeVisible();
  await [1, 2, 3, 4].reduce(async (previous, bay) => {
    await previous;
    await slideAcross(vehicle, sheet.getByRole('button', { name: `Slide to drop bay ${bay}` }));
    await expect(sheet.getByText(new RegExp(`^Bay ${bay} dropped \\d+ s ago$`))).toBeVisible({ timeout: 30_000 });
  }, Promise.resolve());
  await expect(sheet.getByText(/still open/)).toBeHidden();
  await vehicle.keyboard.press('Escape');
  await expectBaysReleased(vehicle);
});

test('dropping bays quickly from the deck shows each bay as it drops, never one left queued by an earlier drop', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await fourServoBays(vehicle);
  await takeOff(vehicle, '10 m');
  await expect(vehicle.getByRole('button', { name: 'Slide to land' })).toBeVisible({ timeout: 60_000 });
  const notice = vehicle.getByRole('status').filter({ hasText: / dropped$/ });
  await [1, 2, 3].reduce(async (previous, bay) => {
    await previous;
    await vehicle.getByRole('button', { name: `Slide to drop ${bay}`, exact: true }).click();
    await slideAcross(vehicle, vehicle.getByRole('button', { name: /^Slide to drop/ }));
    await expect(notice, `the notice on screen after dropping bay ${bay} names bay ${bay}`).toHaveText([`Bay ${bay} dropped`], { timeout: 1_500 });
  }, Promise.resolve());
});

test('a planned four-drop mission on servo bays uploads, flies and drops each bay once', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await fourServoBays(vehicle);
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await vehicle.getByRole('button', { name: 'Waypoint 2' }).first().click();
  const drop = vehicle.getByRole('button', { name: 'Drop', exact: true });
  await [1, 2, 3, 4].reduce(async (previous) => {
    await previous;
    const before = await itemCount(vehicle);
    await drop.click();
    await expect.poll(() => itemCount(vehicle), { timeout: 30_000 }).toBe(before + 1);
  }, Promise.resolve());
  await expect(vehicle.getByText('Drop bay 4').first()).toBeVisible();
  await vehicle.getByRole('button', { name: 'Upload', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: 'Upload', exact: true })).toBeHidden({ timeout: 30_000 });
  await vehicle.getByRole('button', { name: 'Fly', exact: true }).last().click();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to start mission' }));
  await vehicle.getByRole('button', { name: 'More', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Payload').click();
  const sheet = vehicle.getByRole('dialog');
  await [1, 2, 3, 4].reduce(async (previous, bay) => {
    await previous;
    await expect(sheet.getByRole('button', { name: `Bay ${bay} empty, slide to drop` })).toBeVisible({ timeout: 120_000 });
  }, Promise.resolve());
  await vehicle.keyboard.press('Escape');
  await expectBaysReleased(vehicle);
});

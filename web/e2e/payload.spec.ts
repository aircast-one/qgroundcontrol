import { connectMock, expect, flightMode, itemCount, openPlanAtVehicle, slideAcross, takeOff, tapPlanMission, test, vehicleStatus } from './fixtures';

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
  await expect(panel).toBeVisible();
  await panel.getByRole('button', { name: 'Turn on auto-close' }).click();
  await expect(vehicle.getByText(STAYS_OPEN)).toBeHidden({ timeout: 30_000 });
  await vehicle.keyboard.press('Escape');
  await expect(vehicle.getByText('Start mission', { exact: true })).toBeVisible({ timeout: 30_000 });
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
});

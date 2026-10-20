import { type Locator } from '@playwright/test';
import { connectMock, expect, slideAcross, test, vehicleStatus } from './fixtures';

test('the status sheet arms the aircraft without starting layout editing', async ({ vehicle }) => {
  const sheet = await vehicleStatus(vehicle);
  await slideAcross(vehicle, sheet.getByRole('button', { name: 'Slide to arm' }));
  await expect(vehicle.getByText('Arm the vehicle.')).toBeVisible();
  await expect(vehicle.getByRole('button', { name: 'Reset layout' })).toBeHidden();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to arm' }));
  await expect(vehicle.getByRole('button', { name: /Change flight mode$/ }).first()).toContainText('Armed');
});

test('the status sheet lists sensor health and opens the message log', async ({ vehicle }) => {
  const sheet = await vehicleStatus(vehicle);
  await sheet.getByRole('button', { name: 'Show 1 more' }).click();
  await expect(sheet.getByRole('button', { name: 'GPS Normal' }), 'the simulated vehicle reported its GPS switched off over a 12-satellite fix').toBeVisible();
  await sheet.getByRole('button', { name: 'Messages', exact: true }).click();
  const log = vehicle.getByRole('dialog').filter({ hasText: 'messages from the vehicle' });
  await expect(log).toBeVisible();
  await log.getByRole('button', { name: 'Close' }).click();
  await expect(log).toBeHidden();
});

test('status text from the vehicle fills the message log and clear empties it', async ({ app }) => {
  await connectMock(app, ['Send status text + voice']);
  const sheet = await vehicleStatus(app);
  await sheet.getByRole('button', { name: 'Messages', exact: true }).click();
  const log = app.getByRole('dialog').filter({ hasText: /errors? · \d+ warnings?/ });
  await expect(log.getByText('Status text critical')).toBeVisible();
  await expect(log.getByText('Status text info')).toBeVisible();
  await log.getByRole('button', { name: 'Clear' }).click();
  await expect(log).toBeHidden();
  const again = await vehicleStatus(app);
  await again.getByRole('button', { name: 'Messages', exact: true }).click();
  await expect(app.getByText('The vehicle has not said anything yet.')).toBeVisible();
});

test('at desktop width the status pill shows battery time, current draw and satellites', async ({ vehicle }) => {
  await expect(vehicle.getByRole('button', { name: /^\d+:\d\d flight time left$/ })).toBeVisible();
  await expect(vehicle.getByRole('button', { name: /^[\d.]+A current draw$/ })).toBeVisible();
  await expect(vehicle.getByText(/^\d+$/).filter({ hasText: '12' }).first()).toBeVisible();
});

test('all status lists the gimbal and Tilt 90 points it as far down as it goes', async ({ app }) => {
  await connectMock(app, ['Enable gimbal']);
  await app.getByRole('button', { name: 'All status' }).click();
  const gimbal = app.getByRole('button', { name: /· P: -?[\d.]+ ·/ });
  await expect(gimbal).toBeVisible({ timeout: 30_000 });
  const left = async (locator: Locator) => (await locator.boundingBox())?.x ?? Number.NaN;
  expect(Math.abs((await left(app.getByText(/^ESC \d+ OK$/))) - (await left(gimbal)))).toBeLessThan(2);
  await gimbal.click();
  await app.getByRole('button', { name: 'Tilt 90' }).click();
  await expect(app.getByRole('button', { name: /· P: -90\.0 ·/ })).toBeVisible({ timeout: 30_000 });
});

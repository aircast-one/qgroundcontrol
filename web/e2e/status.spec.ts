import { expect, slideAcross, test, vehicleStatus } from './fixtures';

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
  await expect(sheet.getByRole('button', { name: 'GPS Disabled' })).toBeVisible();
  await sheet.getByRole('button', { name: 'Messages' }).click();
  const log = vehicle.getByRole('dialog').filter({ hasText: 'messages from the vehicle' });
  await expect(log).toBeVisible();
  await log.getByRole('button', { name: 'Close' }).click();
  await expect(log).toBeHidden();
});

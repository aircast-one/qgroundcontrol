import { expect, flightMode, openPlanAtVehicle, slideAcross, tapPlanMission, test } from './fixtures';

test.describe.configure({ timeout: 180_000 });

test('an uploaded plan flies as a mission with a progress card', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await vehicle.getByRole('button', { name: 'Upload', exact: true }).click();
  const fly = vehicle.getByRole('button', { name: 'Fly', exact: true }).last();
  await expect(vehicle.getByRole('button', { name: 'Upload', exact: true })).toBeHidden({ timeout: 30_000 });
  await fly.click();
  await expect(vehicle.getByText('Start mission', { exact: true })).toBeVisible();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to start mission' }));
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Mission/, { timeout: 30_000 });
  await expect(vehicle.getByText(/^To waypoint \d+ of \d+/)).toBeVisible({ timeout: 30_000 });
  await expect(vehicle.getByRole('button', { name: 'Skip' })).toBeVisible();
});

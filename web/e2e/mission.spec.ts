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

test('skip moves a running mission on to the next waypoint', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await vehicle.getByRole('button', { name: 'Upload', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: 'Upload', exact: true })).toBeHidden({ timeout: 30_000 });
  await vehicle.getByRole('button', { name: 'Fly', exact: true }).last().click();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to start mission' }));
  const progress = vehicle.getByText(/^To waypoint \d+ of \d+/);
  await expect(progress).toBeVisible({ timeout: 30_000 });
  const [, before] = (await progress.textContent())?.match(/^To waypoint (\d+)/) ?? [];
  await vehicle.getByRole('button', { name: 'Skip' }).click();
  await expect(vehicle.getByText(/^Adjust current waypoint to \d+$/)).toBeVisible();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to set waypoint' }));
  await expect.poll(async () => Number((await progress.textContent())?.match(/^To waypoint (\d+)/)?.[1]), { timeout: 30_000 }).toBeGreaterThan(Number(before));
});

test('landing out of a flown mission says the flight plan is complete', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await vehicle.getByRole('button', { name: 'Upload', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: 'Upload', exact: true })).toBeHidden({ timeout: 30_000 });
  await vehicle.getByRole('button', { name: 'Fly', exact: true }).last().click();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to start mission' }));
  await expect(vehicle.getByText(/^To waypoint \d+ of \d+/)).toBeVisible({ timeout: 30_000 });
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to land' }));
  const done = vehicle.getByRole('dialog').filter({ hasText: 'Flight plan complete' });
  await expect(done).toBeVisible({ timeout: 120_000 });
  await done.getByRole('button', { name: 'Close' }).click();
  await expect(done).toBeHidden();
});

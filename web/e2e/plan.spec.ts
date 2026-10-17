import { expect, openMenu, openPlanAtVehicle, tapPlanMission, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('tapping the plan map builds a mission with home, takeoff and waypoints', async ({ app }) => {
  await openMenu(app, 'Plan');
  await expect(app.getByText('Tap the map to add a waypoint')).toBeVisible();
  await tapPlanMission(app);
  await expect(app.getByText(/^5 items · \d+ m/)).toBeVisible();
  await app.getByRole('button', { name: 'Undo' }).click();
  await expect(app.getByText(/^(Empty plan|[1-4] items · .*)$/)).toBeVisible();
});

test('upload sends the mission to the vehicle and offers to fly it', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await vehicle.getByRole('button', { name: 'Upload', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: 'Upload', exact: true })).toBeHidden({ timeout: 30_000 });
  await expect(vehicle.getByRole('button', { name: 'Fly', exact: true }).last()).toBeVisible();
});

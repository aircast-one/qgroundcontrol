import { expect, itemCount, openMenu, openPlanAtVehicle, tapPlanMission, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('tapping the plan map builds a mission with home, takeoff and waypoints', async ({ app }) => {
  await openMenu(app, 'Plan');
  await expect(app.getByText('Tap the map to add a waypoint')).toBeVisible();
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  const drawn = await itemCount(app);
  await app.getByRole('button', { name: 'Undo' }).click();
  await expect.poll(() => itemCount(app)).toBeLessThan(drawn);
});

test('upload sends the mission to the vehicle and offers to fly it', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await vehicle.getByRole('button', { name: 'Upload', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: 'Upload', exact: true })).toBeHidden({ timeout: 30_000 });
  await expect(vehicle.getByRole('button', { name: 'Fly', exact: true }).last()).toBeVisible();
});

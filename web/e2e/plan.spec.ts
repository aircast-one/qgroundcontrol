import { expect, itemCount, openMenu, openPlanAtVehicle, tapPlanMission, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('tapping the plan map builds a mission with home, takeoff and waypoints', async ({ app }) => {
  await openMenu(app, 'Plan');
  await expect(app.getByText('Click the map to add a waypoint')).toBeVisible();
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  const drawn = await itemCount(app);
  await app.getByRole('button', { name: 'Undo' }).click();
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBeLessThan(drawn);
});

test('upload sends the mission to the vehicle and offers to fly it', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await vehicle.getByRole('button', { name: 'Upload', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: 'Upload', exact: true })).toBeHidden({ timeout: 30_000 });
  await expect(vehicle.getByRole('button', { name: 'Fly', exact: true }).last()).toBeVisible();
});

test('the selected waypoint editor steps its altitude into the item list', async ({ app }) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.getByRole('button', { name: 'Waypoint 3' }).click();
  await expect(app.getByRole('button', { name: /^3 Waypoint 50\.0 m/ })).toBeVisible();
  await app.getByRole('button', { name: 'Increase Altitude' }).click();
  await app.getByRole('button', { name: 'Increase Altitude' }).click();
  await expect(app.getByRole('button', { name: /^3 Waypoint 52\.0 m/ })).toBeVisible({ timeout: 15_000 });
  await app.getByRole('slider', { name: 'Altitude' }).press('ArrowLeft');
  await expect(app.getByRole('button', { name: /^3 Waypoint 51\.0 m/ })).toBeVisible({ timeout: 15_000 });
});

test('the terrain profile charts the mission over the ground at the vehicle', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await expect.poll(() => itemCount(vehicle)).toBeGreaterThanOrEqual(5);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  const chart = vehicle.getByRole('img', { name: / · \d+ m/ });
  await expect(chart).toBeVisible({ timeout: 30_000 });
  await expect(chart).not.toHaveAccessibleName(/ground height/);
});

test('a waypoint deletes from the plan through its editor', async ({ app }) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await app.getByRole('button', { name: /^Advanced/, expanded: false }).last().click();
  const before = await itemCount(app);
  await app.getByRole('button', { name: 'Delete from the plan' }).click();
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBe(before - 1);
});

test('a waypoint takes a hover action and gives it back', async ({ app }) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await app.getByRole('button', { name: 'Add', exact: true }).click();
  await app.getByRole('menuitem', { name: 'Hover' }).click();
  await app.getByRole('dialog').filter({ hasText: 'Hover' }).getByRole('button', { name: 'Done' }).click();
  await expect(app.getByRole('button', { name: /^3 Waypoint .* · holds 5 s/ })).toBeVisible({ timeout: 15_000 });
  await app.getByRole('button', { name: 'Remove Hover' }).click();
  await expect(app.getByRole('button', { name: /^3 Waypoint .* · holds/ })).toHaveCount(0, { timeout: 15_000 });
});

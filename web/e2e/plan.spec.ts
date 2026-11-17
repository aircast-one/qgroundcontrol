import type { Page } from '@playwright/test';
import { connectMock, expect, itemCount, openMenu, openPlanAtVehicle, tapPlanMission, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

const planMenu = async (page: Page, item: string) => {
  await page.getByRole('button', { name: 'Plan menu' }).click();
  await page.getByRole('menuitem', { name: item }).click();
};

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
  const drawn = await itemCount(app);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.keyboard.press('ControlOrMeta+z');
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBeLessThan(drawn);
  await app.keyboard.press('ControlOrMeta+Shift+z');
  await expect.poll(() => itemCount(app), { timeout: 30_000, message: 'an undo and redo close the burst of taps, so each delete below is its own undo step' }).toBe(drawn);
  await app.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await app.getByRole('button', { name: 'Delete waypoint 3' }).click();
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBe(drawn - 1);
  await planMenu(app, 'Undo');
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBe(drawn);
  await planMenu(app, 'Redo');
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBe(drawn - 1);
  await app.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await app.keyboard.down('Delete');
  await app.keyboard.down('Delete');
  await app.keyboard.up('Delete');
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBe(drawn - 2);
  await expect(app.getByRole('button', { name: 'Waypoint 3' }).first(), 'a held Delete removed one waypoint, not two').toBeVisible();
  await app.keyboard.press('ControlOrMeta+z');
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBe(drawn - 1);
});

test('a waypoint moves down the list and back up', async ({ app }) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await app.getByRole('button', { name: 'Increase Altitude' }).click();
  await expect(app.getByRole('button', { name: /^3 Waypoint 51\.0 m/ })).toBeVisible({ timeout: 15_000 });
  await app.getByRole('button', { name: 'Move waypoint 3 down' }).click();
  await expect(app.getByRole('button', { name: /^4 Waypoint 51\.0 m/ })).toBeVisible({ timeout: 15_000 });
  await app.getByRole('button', { name: 'Move waypoint 4 up' }).click();
  await expect(app.getByRole('button', { name: /^3 Waypoint 51\.0 m/ })).toBeVisible({ timeout: 15_000 });
  await app.getByRole('button', { name: /^2 Waypoint / }).click();
  await expect(app.getByRole('button', { name: 'Move waypoint 2 down' })).toBeVisible();
  await expect(app.getByRole('button', { name: 'Move waypoint 2 up' }), 'nothing trades places with the takeoff').toHaveCount(0);
  await app.getByRole('button', { name: /^1 Takeoff/ }).click();
  await expect(app.getByRole('button', { name: /^Move takeoff/ }), 'the takeoff keeps its place').toHaveCount(0);
});

test('a typed altitude past the limit says it was capped', async ({ app }) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await app.getByRole('button', { name: /^Altitude: / }).click();
  const entry = app.getByRole('dialog').filter({ hasText: 'Altitude' });
  await entry.getByRole('textbox').fill('900');
  await expect(entry.getByText('Capped at 500 m, the highest allowed.')).toBeVisible();
  await entry.getByRole('button', { name: 'Set' }).click();
  await expect(app.getByRole('button', { name: /^3 Waypoint 500 m/ })).toBeVisible({ timeout: 15_000 });
});

test('switching a waypoint to AMSL keeps its height above the launch', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await expect.poll(() => itemCount(vehicle)).toBeGreaterThanOrEqual(5);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await vehicle.getByRole('button', { name: /^Relative to launch$/i }).click();
  await vehicle.getByRole('menuitem', { name: /Mixed modes/i }).click();
  const launch = vehicle.getByRole('button', { name: /Launch -?[\d,.]+ m AMSL/ });
  await expect(launch).toBeVisible({ timeout: 30_000 });
  const ground = Number((await launch.innerText()).match(/Launch\s+(-?[\d,.]+) m AMSL/)?.[1]?.replace(',', ''));
  expect(ground, 'the launch has a real elevation, so the conversion is visible').toBeGreaterThan(0);
  await vehicle.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await expect(vehicle.getByRole('button', { name: /^3 Waypoint 50\.0 m/ })).toBeVisible();
  await vehicle.getByRole('button', { name: 'AMSL', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: new RegExp(`^3 Waypoint ${Math.round(ground + 50)}(\\.\\d)? m AMSL`) })).toBeVisible({ timeout: 15_000 });
  const valley = Math.round(ground - 25);
  await vehicle.getByRole('button', { name: /^Altitude: / }).click();
  const entry = vehicle.getByRole('dialog').filter({ hasText: 'Altitude' });
  await entry.getByRole('textbox').fill(`${valley}`);
  await expect(entry.getByText(/Raised to|Capped at/), 'a waypoint below the launch is a normal valley plan, not a value to raise').toHaveCount(0);
  await entry.getByRole('button', { name: 'Set' }).click();
  await expect(vehicle.getByRole('button', { name: new RegExp(`^3 Waypoint ${valley}(\\.0)? m AMSL`) })).toBeVisible({ timeout: 15_000 });
  await vehicle.getByRole('button', { name: 'Decrease Altitude' }).click();
  await expect(vehicle.getByRole('button', { name: new RegExp(`^3 Waypoint ${valley - 1}(\\.0)? m AMSL`) }), 'minus moves the waypoint down, not up to the launch').toBeVisible({ timeout: 15_000 });
  await vehicle.getByRole('button', { name: 'Relative to launch', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: /^3 Waypoint -2\d(\.\d)? m/ }), 'back relative to the launch, it stays below it').toBeVisible({ timeout: 15_000 });
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

test('a plan the connected vehicle cannot fly says which items, and is not converted', async ({ app }) => {
  await openMenu(app, 'Plan');
  await app.getByRole('button', { name: /^Vehicle, / }).click();
  await app.getByRole('menuitem', { name: 'VTOL' }).click();
  await expect(app.getByText(/For .*VTOL/)).toBeVisible();
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.addLocatorHandler(app.getByRole('dialog').filter({ hasText: 'Aircraft connected' }), (prompt) => prompt.getByRole('button', { name: 'Keep my route' }).click());
  await app.getByRole('navigation').getByRole('button', { name: 'Fly', exact: true }).click();
  await connectMock(app);
  await openMenu(app, 'Plan');
  await app.getByRole('button', { name: 'Upload', exact: true }).click();
  const far = app.getByRole('dialog').filter({ hasText: 'from the aircraft' });
  const mismatch = app.getByRole('dialog').filter({ hasText: 'different firmware' });
  await expect(far.or(mismatch)).toBeVisible();
  if (await far.isVisible()) await far.getByRole('button', { name: 'Upload anyway' }).click();
  await mismatch.getByRole('button', { name: 'Convert to this vehicle' }).click();
  await expect(app.getByText(/^The connected vehicle cannot fly item 1 \(VTOL takeoff\)/)).toBeVisible();
  await expect(app.getByText('Plan converted to this vehicle. Check it, then upload.')).toHaveCount(0);
  await app.getByRole('button', { name: 'Upload', exact: true }).click();
  if (await far.isVisible()) await far.getByRole('button', { name: 'Upload anyway' }).click();
  await expect(mismatch, 'the plan still holds the VTOL takeoff, so it still differs from the vehicle').toBeVisible();
});

test('a plan made for another firmware converts to the connected vehicle', async ({ app }) => {
  await openMenu(app, 'Plan');
  await app.getByRole('radiogroup', { name: 'Firmware' }).getByRole('radio', { name: 'ArduPilot' }).click();
  await expect(app.getByText(/For ArduPilot/)).toBeVisible();
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.addLocatorHandler(app.getByRole('dialog').filter({ hasText: 'Aircraft connected' }), (prompt) => prompt.getByRole('button', { name: 'Keep my route' }).click());
  await app.getByRole('navigation').getByRole('button', { name: 'Fly', exact: true }).click();
  await connectMock(app);
  await openMenu(app, 'Plan');
  await app.getByRole('button', { name: 'Upload', exact: true }).click();
  const far = app.getByRole('dialog').filter({ hasText: 'from the aircraft' });
  const mismatch = app.getByRole('dialog').filter({ hasText: 'different firmware' });
  await expect(far.or(mismatch)).toBeVisible();
  if (await far.isVisible()) await far.getByRole('button', { name: 'Upload anyway' }).click();
  await mismatch.getByRole('button', { name: 'Convert to this vehicle' }).click();
  await expect(app.getByText('Plan converted to this vehicle. Check it, then upload.')).toBeVisible();
  await expect(app.getByText(/For PX4/)).toBeVisible({ timeout: 15_000 });
});

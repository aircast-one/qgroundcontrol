import { connectMock, expect, openMenu, openPlanAtVehicle, tapPlanMission, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('a survey pattern computes its area, photo count and interval', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await vehicle.getByRole('button', { name: 'Pattern', exact: true }).click();
  await vehicle.getByRole('dialog').filter({ hasText: 'Which pattern?' }).getByRole('button', { name: 'Survey', exact: true }).click();
  await expect(vehicle.getByText('AREA', { exact: true })).toBeVisible();
  await expect(vehicle.getByText('PHOTOS', { exact: true })).toBeVisible();
  await expect(vehicle.getByText(/^Survey$/).first()).toBeVisible();
  await expect(vehicle.getByText(/[\d,]+ m² · [\d,]+ photos/)).toBeVisible();
});

test('a fence polygon starts as a four-corner inclusion zone and can become keep-out', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await vehicle.getByRole('button', { name: 'Fence', exact: true }).click();
  await vehicle.getByRole('button', { name: 'Polygon', exact: true }).click();
  await expect(vehicle.getByText(/Polygon 1 · Inclusion · 4 vertices/)).toBeVisible();
  await vehicle.getByRole('button', { name: 'Make keep-out' }).click();
  await expect(vehicle.getByText(/Polygon 1 · Exclusion · 4 vertices/)).toBeVisible();
});

test('the corridor scan template starts a corridor pattern', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await vehicle.getByRole('button', { name: 'Templates' }).click();
  await vehicle.getByRole('dialog').filter({ hasText: 'New plan' }).getByText('Corridor scan', { exact: true }).click();
  const create = vehicle.getByRole('dialog').filter({ hasText: 'Start a new plan?' });
  if (await create.isVisible().catch(() => false)) await create.getByRole('button', { name: 'Start new plan' }).click();
  await expect(vehicle.getByRole('button', { name: /Corridor scan .*m² · [\d,]+ photos/ })).toBeVisible();
});

test('tapping the map in Rally mode places a rally point', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await vehicle.getByRole('button', { name: 'Rally', exact: true }).click();
  await expect(vehicle.getByText(/No rally points/)).toBeVisible();
  const box = await vehicle.getByRole('region', { name: 'Map' }).boundingBox();
  if (box === null) throw new Error('plan map is not on screen');
  await expect(async () => {
    await vehicle.mouse.click(box.x + box.width * 0.6, box.y + box.height * 0.45);
    await expect(vehicle.getByText(/No rally points/)).toBeHidden({ timeout: 3_000 });
  }).toPass({ timeout: 30_000 });
  await expect(vehicle.getByText(/Rally point 1|R1|1 rally/).first()).toBeVisible();
  await vehicle.mouse.click(box.x + box.width * 0.7, box.y + box.height * 0.45);
  await expect(vehicle.getByText('Rally point 2', { exact: true })).toBeVisible({ timeout: 15_000 });
  await expect(vehicle.getByRole('textbox', { name: /^Alt / }), 'a new rally point starts at the return altitude, not on the ground').not.toHaveValue(/^0?$/);
});

test('templates wait until the map is zoomed in to a place', async ({ app }) => {
  await openMenu(app, 'Plan');
  await app.getByRole('button', { name: 'Templates' }).click();
  const dialog = app.getByRole('dialog').filter({ hasText: 'New plan' });
  await expect(dialog.getByText(/Zoom the map in to where the mission should go/)).toBeVisible();
  await expect(dialog.getByRole('button', { name: 'Survey', exact: true })).toBeDisabled();
  await expect(dialog.getByRole('button', { name: 'Blank mission' })).toBeEnabled();
});

test('moving a far plan to the aircraft takes its rally points along', async ({ app }) => {
  await openMenu(app, 'Plan');
  await app.getByRole('button', { name: 'Center', exact: true }).click();
  await app.getByRole('menuitem', { name: 'Coordinates…' }).click();
  const centre = app.getByRole('dialog').filter({ hasText: 'Center map on coordinate' });
  await centre.getByRole('textbox', { name: 'Latitude' }).fill('-35.3632');
  await centre.getByRole('textbox', { name: 'Longitude' }).fill('149.1652');
  await centre.getByRole('button', { name: 'Center' }).click();
  await expect(app.locator('span.text-label-small').getByText(/^\d+ (m|km)$/)).toHaveText(/^\d+ m$/, { timeout: 30_000 });
  await tapPlanMission(app);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.getByRole('button', { name: 'Rally', exact: true }).click();
  const box = await app.getByRole('region', { name: 'Map' }).boundingBox();
  if (box === null) throw new Error('plan map is not on screen');
  await expect(async () => {
    await app.mouse.click(box.x + box.width * 0.75, box.y + box.height * 0.3);
    await expect(app.getByText('Rally point 1', { exact: true })).toBeVisible({ timeout: 3_000 });
  }).toPass({ timeout: 30_000 });
  await expect(app.getByRole('textbox', { name: 'Latitude' })).toHaveValue(/^-35\./);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.addLocatorHandler(app.getByRole('dialog').filter({ hasText: 'Aircraft connected' }), (prompt) => prompt.getByRole('button', { name: 'Keep my route' }).click());
  await app.getByRole('navigation').getByRole('button', { name: 'Fly', exact: true }).click();
  await connectMock(app);
  await openMenu(app, 'Plan');
  await app.getByRole('button', { name: 'Upload', exact: true }).click();
  await app.getByRole('dialog').filter({ hasText: 'from the aircraft' }).getByRole('button', { name: 'Move it to the aircraft' }).click();
  await expect(app.getByText('Plan moved to the aircraft')).toBeVisible({ timeout: 15_000 });
  await app.getByRole('button', { name: 'Rally', exact: true }).click();
  await app.getByRole('button', { name: /^Rally 1/ }).click();
  await expect(app.getByRole('textbox', { name: 'Latitude' }), 'the rally point came along to the aircraft').toHaveValue(/^47\./);
});

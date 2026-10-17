import { expect, openPlanAtVehicle, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('a survey pattern computes its area, photo count and interval', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await vehicle.getByRole('button', { name: 'Pattern', exact: true }).click();
  await vehicle.getByRole('dialog').filter({ hasText: 'Which pattern?' }).getByRole('button', { name: 'Survey', exact: true }).click();
  await expect(vehicle.getByText('AREA', { exact: true })).toBeVisible();
  await expect(vehicle.getByText('PHOTOS', { exact: true })).toBeVisible();
  await expect(vehicle.getByText(/^Survey$/).first()).toBeVisible();
  await expect(vehicle.getByText(/\d+ m² · \d+ photos/)).toBeVisible();
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
  const create = vehicle.getByRole('dialog').filter({ hasText: 'Create Plan' });
  if (await create.isVisible().catch(() => false)) await create.getByRole('button', { name: 'Create plan' }).click();
  await expect(vehicle.getByRole('button', { name: /Corridor scan .*m² · \d+ photos/ })).toBeVisible();
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
});

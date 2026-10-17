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

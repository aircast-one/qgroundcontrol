import { readFile } from 'node:fs/promises';
import { expect, flightMode, openMenu, slideAcross, takeOff, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('Analyze › Flight logs downloads a vehicle log and saves it as a file in one step', async ({ vehicle }, testInfo) => {
  await openMenu(vehicle, 'Analyze');
  await vehicle.getByRole('button', { name: /^Flight logs/ }).click();
  await expect(vehicle.getByText('On the vehicle', { exact: true })).toBeVisible();
  const download = vehicle.getByRole('button', { name: /^Download/ });
  await expect(download).toBeDisabled();
  const log = vehicle.getByRole('checkbox', { name: /^Log \d+/ }).first();
  await log.click();
  await expect(log).toHaveAttribute('aria-checked', 'true');
  const saving = vehicle.waitForEvent('download', { timeout: 60_000 });
  await vehicle.getByRole('button', { name: 'Download (1)' }).click();
  const saved = await saving;
  expect(saved.suggestedFilename()).toMatch(/^log_\d+_.+\.(bin|ulg|px4log)$/);
  const file = testInfo.outputPath(saved.suggestedFilename());
  await saved.saveAs(file);
  const bytes = await readFile(file);
  expect(bytes.length).toBeGreaterThan(0);
  expect(bytes.some((byte) => byte !== 0)).toBe(true);
  await expect(vehicle.getByText('On the vehicle', { exact: true })).toBeVisible();
  await expect(vehicle.getByRole('button', { name: /^Save log \d+ again$/ })).toBeVisible();
  await expect(vehicle.getByText(/^Saved to/)).toHaveCount(0);
});

test('a landed flight is reviewed from Analyze, which disconnects the aircraft and plays the flight back on Fly', async ({ vehicle }) => {
  await takeOff(vehicle, '5 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to land' }));
  await expect(vehicle.getByRole('button', { name: 'Take off' }), 'landed and disarmed').toBeVisible({ timeout: 90_000 });
  await openMenu(vehicle, 'Analyze');
  await vehicle.getByRole('button', { name: /^Flight logs/ }).click();
  await expect(vehicle.getByText('Recorded by this app', { exact: true })).toBeVisible();
  await vehicle.getByRole('button', { name: 'Review last flight', exact: true }).click();
  await expect(flightMode(vehicle), 'the flight plays back as a vehicle on Fly').toBeVisible({ timeout: 60_000 });
  await openMenu(vehicle, 'Analyze');
  await vehicle.getByRole('button', { name: /^Flight logs/ }).click();
  await expect(vehicle.getByText(/^Flight on \d{4}-\d{2}-\d{2} at \d{2}:\d{2}$/), 'the flight just flown was recorded in the browser and listed for review').toHaveCount(1);
});

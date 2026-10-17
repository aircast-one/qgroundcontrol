import { readFile } from 'node:fs/promises';
import { expect, openMenu, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('Analyze › Flight logs downloads a vehicle log into the browser and saves it as a file', async ({ vehicle }, testInfo) => {
  await openMenu(vehicle, 'Analyze');
  await vehicle.getByRole('button', { name: /^Flight logs/ }).click();
  await expect(vehicle.getByText('On the vehicle', { exact: true })).toBeVisible();
  const log = vehicle.getByRole('checkbox', { name: /^Log \d+/ }).first();
  await log.click();
  await expect(log).toHaveAttribute('aria-checked', 'true');
  await vehicle.getByRole('button', { name: 'Download (1)' }).click();
  await expect(vehicle.getByText('In this browser', { exact: true })).toBeVisible({ timeout: 60_000 });
  await expect(vehicle.getByText('On the vehicle', { exact: true })).toBeHidden();
  await expect(vehicle.getByText(/^Saved to/)).toHaveCount(0);
  const saving = vehicle.waitForEvent('download');
  await vehicle.getByRole('button', { name: /^Save log \d+$/ }).click();
  const saved = await saving;
  expect(saved.suggestedFilename()).toMatch(/^log_\d+_.+\.(bin|ulg|px4log)$/);
  const file = testInfo.outputPath(saved.suggestedFilename());
  await saved.saveAs(file);
  const bytes = await readFile(file);
  expect(bytes.length).toBeGreaterThan(0);
  expect(bytes.some((byte) => byte !== 0)).toBe(true);
});

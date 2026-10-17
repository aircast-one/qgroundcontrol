import { expect, openAircraftSetup, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('a parameter found by search resets to its default from the editor', async ({ vehicle }) => {
  await openAircraftSetup(vehicle);
  await vehicle.getByRole('button', { name: /^Parameters/ }).click();
  await vehicle.getByRole('textbox', { name: 'Search parameters' }).fill('RTL_RETURN_ALT');
  const row = vehicle.getByRole('button', { name: /^RTL_RETURN_ALT/ }).first();
  await expect(row).toContainText('30.0 m');
  await row.click();
  const editor = vehicle.getByRole('dialog').filter({ hasText: 'RTL_RETURN_ALT' });
  await expect(editor.getByText(/default 60\.0 m/)).toBeVisible();
  await editor.getByRole('button', { name: 'Reset to default' }).click();
  await expect(editor).toBeHidden();
  await expect(row).toContainText('60.0 m');
});

test('a parameter typed into its editor is written to the vehicle', async ({ vehicle }) => {
  await openAircraftSetup(vehicle);
  await vehicle.getByRole('button', { name: /^Parameters/ }).click();
  await vehicle.getByRole('textbox', { name: 'Search parameters' }).fill('RTL_RETURN_ALT');
  const row = vehicle.getByRole('button', { name: /^RTL_RETURN_ALT/ }).first();
  await row.click();
  const editor = vehicle.getByRole('dialog').filter({ hasText: 'RTL_RETURN_ALT' });
  const value = editor.getByRole('textbox', { name: /return altitude/i });
  await value.fill('45');
  await value.press('Enter');
  await editor.getByRole('button', { name: 'Done' }).click();
  await expect(editor).toBeHidden();
  await expect(row).toContainText('45.0 m');
});

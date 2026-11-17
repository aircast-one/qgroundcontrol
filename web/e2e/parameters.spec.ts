import { connectMock, expect, openAircraftSetup, test } from './fixtures';

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

test('a parameter typed into its editor is written to the vehicle on Save, with an undo', async ({ vehicle }) => {
  await openAircraftSetup(vehicle);
  await vehicle.getByRole('button', { name: /^Parameters/ }).click();
  await vehicle.getByRole('textbox', { name: 'Search parameters' }).fill('RTL_RETURN_ALT');
  const row = vehicle.getByRole('button', { name: /^RTL_RETURN_ALT/ }).first();
  await row.click();
  const editor = vehicle.getByRole('dialog').filter({ hasText: 'RTL_RETURN_ALT' });
  await editor.getByRole('textbox', { name: /return altitude/i }).fill('45');
  await editor.getByRole('button', { name: 'Save' }).click();
  await expect(editor).toBeHidden();
  await expect(row).toContainText('45.0 m');
  await vehicle.getByRole('status').getByRole('button', { name: 'Undo' }).click();
  await expect(row).toContainText('30.0 m');
});

test('Enter in the parameter editor runs its Save, so the value is written once with an undo', async ({ vehicle }) => {
  await openAircraftSetup(vehicle);
  await vehicle.getByRole('button', { name: /^Parameters/ }).click();
  await vehicle.getByRole('textbox', { name: 'Search parameters' }).fill('RTL_RETURN_ALT');
  const row = vehicle.getByRole('button', { name: /^RTL_RETURN_ALT/ }).first();
  await row.click();
  const editor = vehicle.getByRole('dialog').filter({ hasText: 'RTL_RETURN_ALT' });
  await editor.getByRole('textbox', { name: /return altitude/i }).fill('45');
  await editor.getByRole('textbox', { name: /return altitude/i }).press('Enter');
  await expect(editor).toBeHidden();
  await expect(row).toContainText('45.0 m');
  await vehicle.getByRole('status').getByRole('button', { name: 'Undo' }).click();
  await expect(row).toContainText('30.0 m');
});

test('a value half-typed into the parameter editor is dropped by Escape, Cancel or a tap outside', async ({ vehicle }) => {
  await openAircraftSetup(vehicle);
  await vehicle.getByRole('button', { name: /^Parameters/ }).click();
  await vehicle.getByRole('textbox', { name: 'Search parameters' }).fill('RTL_RETURN_ALT');
  const row = vehicle.getByRole('button', { name: /^RTL_RETURN_ALT/ }).first();
  const editor = vehicle.getByRole('dialog').filter({ hasText: 'RTL_RETURN_ALT' });
  const value = editor.getByRole('textbox', { name: /return altitude/i });
  await row.click();
  await value.fill('2');
  await value.press('Escape');
  await expect(value).toHaveValue('30');
  await value.fill('2');
  await vehicle.keyboard.press('Escape');
  await vehicle.keyboard.press('Escape');
  await expect(editor).toBeHidden();
  await row.click();
  await value.fill('2');
  await editor.getByRole('button', { name: 'Cancel' }).click();
  await expect(editor).toBeHidden();
  await row.click();
  await value.fill('2');
  await vehicle.mouse.click(5, 5);
  await expect(editor).toBeHidden();
  await expect(row).toContainText('30.0 m');
  await expect(vehicle.getByRole('status').getByRole('button', { name: 'Undo' })).toHaveCount(0);
});

test('an ArduPilot parameter is described by the metadata file the browser fetches for that firmware', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Parameters/ }).click();
  await app.getByRole('textbox', { name: 'Search parameters' }).fill('FLTMODE1');
  await app.getByRole('button', { name: /^FLTMODE1/ }).first().click();
  const editor = app.getByRole('dialog').filter({ hasText: 'FLTMODE1' });
  await expect(editor.getByText('Flight Mode 1', { exact: false }).first()).toBeVisible();
  await expect(editor.getByText(/Flightmode channel/)).toBeVisible();
});

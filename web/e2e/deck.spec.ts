import { expect, showMapFull, test } from './fixtures';

test('more actions lists every deck command', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'More', exact: true }).click();
  const sheet = vehicle.getByRole('dialog').filter({ hasText: 'More actions' });
  await expect(sheet).toBeVisible();
  await expect(sheet.getByText('Choose readings')).toBeVisible();
  await expect(sheet.getByText('Edit layout')).toBeVisible();
  await expect(sheet.getByText('Gripper')).toBeVisible();
});

test('edit layout from more actions starts and ends layout editing', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'More', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Edit layout').click();
  await expect(vehicle.getByRole('button', { name: 'Reset layout' })).toBeVisible();
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await expect(vehicle.getByRole('button', { name: 'Reset layout' })).toBeHidden();
});

test('choose readings opens the readings chooser', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'More', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Choose readings').click();
  await expect(vehicle.getByRole('dialog').filter({ hasNotText: 'More actions' })).toBeVisible();
});

test('map layers switches the base map', async ({ vehicle: app }) => {
  await showMapFull(app);
  await app.getByRole('button', { name: 'Map layers' }).click();
  const layers = app.getByRole('dialog').filter({ hasText: 'Map layers' });
  await layers.getByRole('button', { name: 'Road', exact: true }).click();
  await expect(layers.getByRole('button', { name: 'Road', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await app.keyboard.press('Escape');
  await expect(app.getByText('Bing Road', { exact: true })).toBeVisible();
});

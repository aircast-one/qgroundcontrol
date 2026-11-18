import { connectMock, expect, test } from './fixtures';

test('a vehicle camera records video and blocks photo mode while recording', async ({ app }) => {
  await connectMock(app, ['Enable camera', 'Enable gimbal']);
  const start = app.getByRole('button', { name: 'Start recording' });
  await expect(start).toBeVisible();
  await start.click();
  await expect(app.getByRole('button', { name: 'Stop recording' })).toBeVisible();
  await expect(app.getByText(/^REC \d\d:\d\d:\d\d$/)).toBeVisible();
  await expect(app.getByRole('button', { name: 'Photo', exact: true })).toBeDisabled();
  await app.getByRole('button', { name: 'Stop recording' }).click();
  await expect(start).toBeVisible();
  await expect(app.getByRole('button', { name: 'Photo', exact: true })).toBeEnabled();
});

test('a vehicle camera shows its zoom level', async ({ app }) => {
  await connectMock(app, ['Enable camera']);
  await expect(app.getByRole('button', { name: /^Zoom \d+%$/ })).toBeVisible();
});

test('the zoom chip above the shutter stays clear of the speed readings beside the picture', async ({ app }) => {
  await connectMock(app, ['Enable camera']);
  const zoom = await app.getByRole('button', { name: /^Zoom \d+%$/ }).boundingBox();
  const climb = await app.getByText('V.S', { exact: true }).locator('..').boundingBox();
  expect(zoom).not.toBeNull();
  expect(climb).not.toBeNull();
  expect((climb?.y ?? 0) + (climb?.height ?? 0), 'the lowest speed reading ends above the zoom chip').toBeLessThanOrEqual(zoom?.y ?? 0);
});

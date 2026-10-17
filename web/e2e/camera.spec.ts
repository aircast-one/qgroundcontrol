import { connectMock, expect, test } from './fixtures';

test('a vehicle camera records video and blocks photo mode while recording', async ({ app }) => {
  await connectMock(app, ['Enable camera', 'Enable gimbal']);
  const start = app.getByRole('button', { name: 'Start recording' });
  await expect(start).toBeVisible();
  await start.click();
  await expect(app.getByRole('button', { name: 'Stop recording' })).toBeVisible();
  await expect(app.getByText(/^REC \d\d:\d\d:\d\d$/)).toBeVisible();
  await expect(app.getByRole('button', { name: 'PHOTO', exact: true })).toBeDisabled();
  await app.getByRole('button', { name: 'Stop recording' }).click();
  await expect(start).toBeVisible();
  await expect(app.getByRole('button', { name: 'PHOTO', exact: true })).toBeEnabled();
});

test('a vehicle camera shows its zoom level', async ({ app }) => {
  await connectMock(app, ['Enable camera']);
  await expect(app.getByRole('button', { name: /^Zoom \d+%$/ })).toBeVisible();
});

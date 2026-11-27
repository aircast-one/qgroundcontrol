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

test('a trackpad pinch on the picture steps the camera zoom', async ({ app }) => {
  await connectMock(app, ['Enable camera']);
  const zoom = app.getByRole('button', { name: /^Zoom \d+%$/ });
  await expect(zoom).toHaveText('1%');
  const view = app.viewportSize()!;
  await app.mouse.move(view.width / 2, view.height / 2);
  await app.keyboard.down('Control');
  await app.mouse.wheel(0, -200);
  await app.keyboard.up('Control');
  await expect(zoom, 'pinching out steps the camera in').toHaveText('21%');
});

test('the zoom slider let go at full zoom leaves a slow lens to zoom all the way in', async ({ app }) => {
  await connectMock(app, ['Enable camera']);
  const zoom = app.getByRole('button', { name: /^Zoom \d+%$/ });
  await expect(zoom).toHaveText('1%');
  await zoom.click();
  await app.getByRole('slider', { name: 'Zoom' }).focus();
  await app.keyboard.press('End');
  await expect(zoom, 'the mock lens takes four seconds to zoom in; a reading taken on the way must never go back out as a new zoom').toHaveText('100%', { timeout: 20_000 });
});

test('the zoom chip above the shutter stays clear of the speed readings beside the picture', async ({ app }) => {
  await connectMock(app, ['Enable camera']);
  const zoom = await app.getByRole('button', { name: /^Zoom \d+%$/ }).boundingBox();
  const climb = await app.getByText('V.S', { exact: true }).locator('..').boundingBox();
  expect(zoom).not.toBeNull();
  expect(climb).not.toBeNull();
  expect((climb?.y ?? 0) + (climb?.height ?? 0), 'the lowest speed reading ends above the zoom chip').toBeLessThanOrEqual(zoom?.y ?? 0);
});

test('a photo taken leaves its count under the shutter, so the pilot sees each press land', async ({ app }) => {
  await connectMock(app, ['Enable camera']);
  await app.getByRole('button', { name: 'Photo', exact: true }).click();
  const shutter = app.getByRole('button', { name: 'Take photo' });
  await expect(shutter).toBeEnabled();
  await shutter.click();
  await expect(app.getByText('00001', { exact: true }), 'the first picture is counted').toBeVisible();
  await expect(shutter, 'the camera is ready again').toBeEnabled();
  await expect(app.getByText('00001', { exact: true }), 'and the count stays once the capture is over').toBeVisible();
});

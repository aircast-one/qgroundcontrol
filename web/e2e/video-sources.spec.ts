import { expect, test } from './fixtures';

test('a WHEP video source can be added and is listed', async ({ app }) => {
  await app.getByRole('button', { name: 'Settings', exact: true }).click();
  await app.getByRole('tablist').getByRole('tab', { name: 'Transmission' }).click();
  const row = app.getByRole('button', { name: /^Video sources/ });
  await expect(async () => {
    if (await row.isVisible()) await row.click();
    await expect(app.getByRole('button', { name: 'Add a video source' })).toBeVisible({ timeout: 2_000 });
  }).toPass({ timeout: 30_000 });
  await app.getByRole('button', { name: 'Add a video source' }).click();
  const types = app.getByRole('dialog').filter({ hasText: 'Add video source' });
  await expect(types.getByRole('button', { name: /^RTSP/ })).toBeVisible();
  await types.getByRole('button', { name: /^WebRTC \(WHEP\)/ }).click();
  await app.getByRole('textbox').first().fill('http://localhost:8889/cam/whep');
  await app.getByRole('button', { name: 'Save' }).click();
  await expect(app.getByText('http://localhost:8889/cam/whep')).toBeVisible();
});

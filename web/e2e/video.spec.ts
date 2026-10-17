import { type Page } from '@playwright/test';
import { connectMock, expect, test } from './fixtures';

const WHEP_STREAM = 'http://localhost:18889/e2e/whep';

const addVideoSource = async (page: Page, type: RegExp, address: string) => {
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('tablist').getByRole('tab', { name: 'Transmission' }).click();
  const row = page.getByRole('button', { name: /^Video sources/ });
  await expect(async () => {
    if (await row.isVisible()) await row.click();
    await expect(page.getByRole('button', { name: 'Add a video source' })).toBeVisible({ timeout: 2_000 });
  }).toPass({ timeout: 30_000 });
  await page.getByRole('button', { name: 'Add a video source' }).click();
  await page.getByRole('dialog').filter({ hasText: 'Add video source' }).getByRole('button', { name: type }).click();
  await page.getByRole('textbox').first().fill(address);
  await page.getByRole('button', { name: 'Save' }).click();
  await expect(page.getByText(address)).toBeVisible();
  await page.getByRole('button', { name: 'Close settings' }).click();
};

test('a WHEP camera plays in the Fly view', async ({ app }) => {
  await addVideoSource(app, /^WebRTC \(WHEP\)/, WHEP_STREAM);
  const video = app.locator('video').first();
  await expect.poll(() => video.evaluate((element: HTMLVideoElement) => [element.videoWidth, element.videoHeight]), { timeout: 30_000 }).toEqual([640, 360]);
  const frames = () => video.evaluate((element: HTMLVideoElement) => element.getVideoPlaybackQuality().totalVideoFrames);
  const before = await frames();
  await expect.poll(frames).toBeGreaterThan(before);
  await expect(app.getByText(/^No video/)).toHaveCount(0);
});

test('a stream the browser cannot play says so', async ({ app }) => {
  await addVideoSource(app, /^RTSP/, 'rtsp://localhost:18554/e2e');
  await expect(app.getByText('This browser can only play WebRTC (WHEP) streams.', { exact: false })).toBeVisible({ timeout: 30_000 });
});

test('split view plays a second WHEP camera beside the main one', async ({ app }) => {
  await addVideoSource(app, /^WebRTC \(WHEP\)/, WHEP_STREAM);
  await addVideoSource(app, /^WebRTC \(WHEP\)/, `${WHEP_STREAM}?second`);
  await app.getByRole('button', { name: /^Cameras, showing/ }).click();
  const split = app.getByRole('menuitemcheckbox', { name: /Split view/ });
  await expect(split).toHaveAttribute('aria-checked', 'false');
  await split.click();
  await expect(app.getByRole('button', { name: /^Second camera, / })).toBeVisible({ timeout: 30_000 });
  const playing = () => app.locator('video').evaluateAll((videos: HTMLVideoElement[]) => videos.filter((video) => video.videoWidth > 0).length);
  await expect.poll(playing, { timeout: 30_000 }).toBe(2);
  await app.getByRole('button', { name: /^Cameras, showing/ }).click();
  await expect(app.getByRole('menuitemcheckbox', { name: /Split view/ })).toHaveAttribute('aria-checked', 'true');
});

test('retry on a WHEP stream that never arrives posts a fresh offer', async ({ app }) => {
  const missing = 'http://localhost:18889/missing/whep';
  const offers: string[] = [];
  app.on('request', (request) => {
    if (request.method() === 'POST' && request.url() === missing) offers.push(request.url());
  });
  await addVideoSource(app, /^WebRTC \(WHEP\)/, missing);
  await expect.poll(() => offers.length, { timeout: 30_000 }).toBeGreaterThan(0);
  const retry = app.getByRole('button', { name: 'Retry' });
  await expect(retry).toBeVisible({ timeout: 30_000 });
  await expect(app.getByText('localhost:18889 has no stream at /missing/whep. Check the path.')).toBeVisible();
  const before = offers.length;
  await retry.click();
  await expect.poll(() => offers.length, { timeout: 5_000 }).toBeGreaterThan(before);
});

test('proximity sensors draw distance rings over the live video', async ({ app }) => {
  await addVideoSource(app, /^WebRTC \(WHEP\)/, WHEP_STREAM);
  const video = app.locator('video').first();
  await expect.poll(() => video.evaluate((element: HTMLVideoElement) => element.videoWidth), { timeout: 30_000 }).toBeGreaterThan(0);
  await connectMock(app, ['Enable proximity sensors']);
  await expect(video.locator('xpath=..').getByText(/^\d+\.\d\d$/).first()).toBeVisible({ timeout: 30_000 });
});

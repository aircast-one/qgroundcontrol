import { readFile } from 'node:fs/promises';
import { type Page } from '@playwright/test';
import { connectMock, expect, test } from './fixtures';

const WHEP_STREAM = 'http://localhost:18889/e2e/whep';

const coreView = (page: Page, path: string) =>
  page.evaluate(async ([module, watched]) => ((await import(module)) as { Qgc: { path: (path: string) => unknown } }).Qgc.path(watched), ['/src/bridge/Qgc.ts', path] as const);

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
  await expect.poll(() => coreView(app, 'view.detections'), { timeout: 30_000 }).toMatchObject({ class: 'Detections' });
  expect(await coreView(app, 'view.detections'), 'the browser cannot follow a detector, and said so in a red strip over the minimap').toMatchObject({ available: false, error: null });
  await expect(app.getByText(/^Detections:/)).toHaveCount(0);
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

test('recording the live WHEP video saves a video file in the browser', async ({ app }, testInfo) => {
  await addVideoSource(app, /^WebRTC \(WHEP\)/, WHEP_STREAM);
  const video = app.locator('video').first();
  await expect.poll(() => video.evaluate((element: HTMLVideoElement) => element.videoWidth), { timeout: 30_000 }).toBeGreaterThan(0);
  await app.getByRole('button', { name: 'Start recording' }).first().click();
  const stop = app.getByRole('button', { name: 'Stop recording' }).first();
  await expect(stop).toBeVisible({ timeout: 15_000 });
  await expect(app.getByText(/^REC 00:00:(0[3-9]|[1-5]\d)$/).first()).toBeVisible({ timeout: 30_000 });
  const saving = app.waitForEvent('download');
  await stop.click();
  const saved = await saving;
  expect(saved.suggestedFilename()).toMatch(/\.(mp4|webm)$/);
  const file = testInfo.outputPath(saved.suggestedFilename());
  await saved.saveAs(file);
  expect((await readFile(file)).length).toBeGreaterThan(10_000);
  await expect(app.getByRole('button', { name: 'Start recording' }).first()).toBeVisible();
});

test('a tap on the live picture points the gimbal there', async ({ app }) => {
  await addVideoSource(app, /^WebRTC \(WHEP\)/, WHEP_STREAM);
  await connectMock(app, ['Enable gimbal']);
  const video = app.locator('video').first();
  await expect.poll(() => video.evaluate((element: HTMLVideoElement) => element.videoWidth), { timeout: 30_000 }).toBe(640);
  const pitch = () =>
    app.evaluate(async (module) => {
      const view = (await ((await import(module)) as { Qgc: { get: (path: string) => Promise<unknown> } }).Qgc.get('view.gimbal')) as { gimbals?: { pitch: number | null }[] } | null;
      return view?.gimbals?.[0]?.pitch ?? null;
    }, '/src/bridge/Qgc.ts');
  await expect.poll(pitch, { timeout: 30_000 }).not.toBeNull();
  const before = (await pitch()) as number;
  const box = await video.boundingBox();
  if (box === null) throw new Error('the video has no box');
  await app.mouse.click(box.x + box.width / 2, box.y + box.height * 0.9);
  await expect.poll(pitch, { timeout: 30_000 }).toBeLessThan(before - 5);
});

test('measuring the lens learns how wide the camera sees from how far a detection slides', async ({ app }) => {
  test.setTimeout(240_000);
  const DEVICE = 'http://localhost:18780';
  const LENS = { h: 60, v: 34 };
  await addVideoSource(app, /^WebRTC \(WHEP\)/, WHEP_STREAM);
  await connectMock(app, ['Enable camera', 'Enable gimbal']);
  await app.evaluate(
    async ([module, device]) => {
      const { Qgc } = (await import(module)) as { Qgc: { set: (path: string, value: unknown) => Promise<unknown>; invoke: (path: string, ...args: unknown[]) => Promise<boolean> } };
      await Qgc.set('settings.appSettings.detectionsHttpPort', 18780);
      await Qgc.invoke('device.pair', `${device}/`);
    },
    ['/src/bridge/Qgc.ts', DEVICE] as const,
  );
  const read = (path: string) => app.evaluate(async ([module, watched]) => ((await import(module)) as { Qgc: { get: (path: string) => Promise<unknown> } }).Qgc.get(watched), ['/src/bridge/Qgc.ts', path] as const);
  const aim = async () => ((await read('view.gimbal')) as { gimbals?: { pitch: number | null; bodyYaw: number | null }[] } | null)?.gimbals?.[0] ?? null;
  await expect.poll(async () => (await aim())?.pitch ?? null, { timeout: 60_000 }).not.toBeNull();
  const start = (await aim()) as { pitch: number; bodyYaw: number };
  const sliding = { on: true };
  const slide = (async () => {
    while (sliding.on) {
      const now = await aim().catch(() => null);
      if (now !== null && now.pitch !== null && now.bodyYaw !== null) {
        const scene = { x: 0.4 + (start.bodyYaw - now.bodyYaw) / LENS.h, y: 0.4 + (now.pitch - start.pitch) / LENS.v };
        await fetch(`${DEVICE}/scene`, { method: 'PUT', body: JSON.stringify(scene) });
      }
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
  })();
  await expect(app.getByText('Approximate aim · measure the lens in camera details')).toBeVisible({ timeout: 30_000 });
  await app.getByRole('button', { name: /^Cameras, showing/ }).click();
  await app.getByRole('menuitem', { name: 'Camera settings' }).click();
  await app.getByRole('button', { name: 'Measure the lens' }).click();
  await expect(app.getByText(/· measured$/)).toBeVisible({ timeout: 200_000 });
  sliding.on = false;
  await slide;
  const lens = (await read('view.lens')) as { hFov: number; vFov: number; source: string };
  expect(lens.source).toBe('measured');
  await app.keyboard.press('Escape');
  await expect(app.getByText('Approximate aim · measure the lens in camera details')).toHaveCount(0);
  expect(Math.abs(lens.hFov - LENS.h), `measured ${lens.hFov}°`).toBeLessThan(3);
  expect(Math.abs(lens.vFov - LENS.v), `measured ${lens.vFov}°`).toBeLessThan(3);
  await fetch(`${DEVICE}/scene`, { method: 'PUT', body: JSON.stringify({ x: 0.4, y: 0.4 }) });
});

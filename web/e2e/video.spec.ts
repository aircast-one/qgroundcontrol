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

const holdGimbal = (page: Page, at: { pitch: number; yaw: number }) =>
  page.evaluate(
    async ([module, pitch, yaw]) => ((await import(module)) as { Qgc: { invoke: (path: string, ...args: unknown[]) => Promise<boolean> } }).Qgc.invoke('gimbal.aim', pitch, yaw),
    ['/src/bridge/Qgc.ts', at.pitch, at.yaw] as const,
  );

test('the gimbal sheet chooses what a drag on the video does and opens every gimbal setting', async ({ app }) => {
  await addVideoSource(app, /^WebRTC \(WHEP\)/, WHEP_STREAM);
  await connectMock(app, ['Enable gimbal']);
  const later = app.getByRole('button', { name: 'Fly now, finish loading in background' });
  if (await later.isVisible()) await later.click();
  await app.getByRole('button', { name: /^Cameras, showing/ }).click();
  await app.getByRole('menuitem', { name: 'Gimbal settings' }).click();
  await expect(app.getByText('On the video', { exact: true })).toBeVisible();
  await expect(app.getByRole('button', { name: 'Tap to point' })).toHaveAttribute('aria-pressed', 'true');
  await app.getByRole('button', { name: 'Drag to move' }).click();
  await expect(app.getByRole('button', { name: 'Drag to move' })).toHaveAttribute('aria-pressed', 'true');
  await expect(app.getByText('The camera follows your drag on the video.')).toBeVisible();
  await app.getByRole('button', { name: 'Gimbal settings', exact: true }).click();
  await expect(app.getByText('Show gimbal azimuth indicator in map')).toBeVisible();
});

test('a tap on the live picture turns the gimbal along the ray through that point of the lens', async ({ app }) => {
  await addVideoSource(app, /^WebRTC \(WHEP\)/, WHEP_STREAM);
  await connectMock(app, ['Enable gimbal']);
  const video = app.locator('video').first();
  await expect.poll(() => video.evaluate((element: HTMLVideoElement) => element.videoWidth), { timeout: 30_000 }).toBe(640);
  const aim = () =>
    app.evaluate(async (module) => {
      const view = (await ((await import(module)) as { Qgc: { get: (path: string) => Promise<unknown> } }).Qgc.get('view.gimbal')) as { gimbals?: { pitch: number | null; bodyYaw: number | null }[] } | null;
      const gimbal = view?.gimbals?.[0];
      return gimbal?.pitch == null || gimbal.bodyYaw == null ? null : { pitch: gimbal.pitch, yaw: gimbal.bodyYaw };
    }, '/src/bridge/Qgc.ts');
  await expect.poll(aim, { timeout: 30_000 }).not.toBeNull();
  const later = app.getByRole('button', { name: 'Fly now, finish loading in background' });
  if (await later.isVisible()) await later.click();
  await expect(later).toBeHidden();
  const box = await video.boundingBox();
  if (box === null) throw new Error('the video has no box');
  await holdGimbal(app, (await aim()) as { pitch: number; yaw: number });
  const held = async () => {
    const first = await aim();
    await new Promise((resolve) => setTimeout(resolve, 2_500));
    const second = await aim();
    return first !== null && second !== null && Math.abs(first.pitch - second.pitch) < 0.05 && Math.abs(first.yaw - second.yaw) < 0.05;
  };
  await expect.poll(held, { timeout: 30_000 }).toBe(true);
  const before = (await aim()) as { pitch: number; yaw: number };
  await app.mouse.click(box.x + box.width * 0.75, box.y + box.height * 0.9);
  const [yawOff, tilt] = aimThroughLens(0.5, -0.8, 80, 80, before.pitch);
  const expected = { pitch: Math.max(-45, Math.min(45, tilt)), yaw: before.yaw + yawOff };
  await expect.poll(async () => {
    const now = await aim();
    return now === null ? null : Math.max(Math.abs(now.pitch - expected.pitch), Math.abs(now.yaw - expected.yaw));
  }, { timeout: 30_000, message: `expected ${JSON.stringify(expected)}` }).toBeLessThan(0.5);
});

const aimThroughLens = (pan: number, tilt: number, hFov: number, vFov: number, pitch: number): [number, number] => {
  const right = pan * Math.tan(radians(hFov) / 2);
  const up = tilt * Math.tan(radians(vFov) / 2);
  const [sin, cos] = [Math.sin(radians(pitch)), Math.cos(radians(pitch))];
  const [forward, raised] = [cos - sin * up, sin + cos * up];
  return [(Math.atan2(right, forward) * 180) / Math.PI, (Math.atan2(raised, Math.hypot(forward, right)) * 180) / Math.PI];
};

const radians = (degrees: number) => (degrees * Math.PI) / 180;

const seenThroughLens = (object: { pitch: number; bodyYaw: number }, camera: { pitch: number; bodyYaw: number }, lens: { h: number; v: number }): [number, number] => {
  const world = [Math.cos(radians(object.pitch)) * Math.cos(radians(object.bodyYaw)), Math.cos(radians(object.pitch)) * Math.sin(radians(object.bodyYaw)), Math.sin(radians(object.pitch))];
  const [ys, yc] = [Math.sin(radians(camera.bodyYaw)), Math.cos(radians(camera.bodyYaw))];
  const [x1, y1, z1] = [world[0] * yc + world[1] * ys, -world[0] * ys + world[1] * yc, world[2]];
  const [ps, pc] = [Math.sin(radians(camera.pitch)), Math.cos(radians(camera.pitch))];
  const [forward, raised] = [pc * x1 + ps * z1, -ps * x1 + pc * z1];
  return [0.5 + y1 / forward / (2 * Math.tan(radians(lens.h) / 2)), 0.5 - raised / forward / (2 * Math.tan(radians(lens.v) / 2))];
};

test('measuring the lens learns how wide the camera sees from how far a detection slides', async ({ app }) => {
  test.setTimeout(240_000);
  const DEVICE = 'http://localhost:18780';
  const LENS = { h: 60, v: 34 };
  await addVideoSource(app, /^WebRTC \(WHEP\)/, 'http://localhost:18889/lens/whep');
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
  const wandering = (await aim()) as { pitch: number; bodyYaw: number };
  await holdGimbal(app, { pitch: wandering.pitch, yaw: wandering.bodyYaw });
  await expect.poll(async () => { const now = await aim(); return now !== null && Math.abs((now.pitch ?? 0) - wandering.pitch) < 0.05 && Math.abs((now.bodyYaw ?? 0) - wandering.bodyYaw) < 0.05; }, { timeout: 30_000 }).toBe(true);
  const start = (await aim()) as { pitch: number; bodyYaw: number };
  const sliding = { on: true };
  const slide = (async () => {
    while (sliding.on) {
      const now = await aim().catch(() => null);
      if (now !== null && now.pitch !== null && now.bodyYaw !== null) {
        const [x, y] = seenThroughLens(start, { pitch: now.pitch, bodyYaw: now.bodyYaw }, LENS);
        const scene = { x: x - 0.1, y: y - 0.1 };
        await fetch(`${DEVICE}/scene/lens`, { method: 'PUT', body: JSON.stringify(scene) });
      }
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
  })();
  await expect(app.getByText('Approximate aim · measure the lens in camera details')).toBeVisible({ timeout: 60_000 });
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
  expect(Math.abs(lens.hFov - LENS.h), `measured ${lens.hFov}°`).toBeLessThan(1);
  expect(Math.abs(lens.vFov - LENS.v), `measured ${lens.vFov}°`).toBeLessThan(1);
});

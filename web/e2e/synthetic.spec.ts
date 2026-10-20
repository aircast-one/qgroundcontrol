import { type Page } from '@playwright/test';
import { connectMock, expect, flightMode, slideAcross, takeOff, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('with a vehicle connected the Fly screen draws the 3D view instead of failing', async ({ vehicle }) => {
  await expect(vehicle.getByText('3D view failed to start')).toBeHidden({ timeout: 30_000 });
  const canvas = vehicle.locator('canvas:not(.maplibregl-canvas)').filter({ visible: true });
  await expect.poll(async () => {
    const boxes = await canvas.evaluateAll((all) => all.map((element) => element.getBoundingClientRect().width * element.getBoundingClientRect().height));
    return Math.max(0, ...boxes);
  }, { timeout: 30_000 }).toBeGreaterThan(400 * 300);
});

test('the synthetic view stays on screen in flight with no camera, and comes back from the map', async ({ vehicle }) => {
  const map = vehicle.getByRole('button', { name: 'Map', exact: true });
  await expect(map).toBeVisible({ timeout: 30_000 });
  await takeOff(vehicle, '10 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^(Takeoff|Hold) Flying/, { timeout: 30_000 });
  await expect(map).toBeVisible();
  await map.click();
  const back = vehicle.getByRole('button', { name: 'Video', exact: true });
  await expect(vehicle.getByText('Synthetic view', { exact: true })).toBeVisible();
  await back.click();
  await expect(map).toBeVisible();
});

const syntheticCanvas = async (page: Page) => {
  const later = page.getByRole('button', { name: 'Fly now, finish loading in background' });
  if (await later.isVisible()) await later.click();
  const canvas = page.locator('canvas:not(.maplibregl-canvas)').filter({ visible: true }).first();
  await expect(canvas).toBeVisible({ timeout: 30_000 });
  return (await canvas.boundingBox())!;
};

const read = (page: Page, path: string) => page.evaluate(async ([module, watched]) => ((await import(module)) as { Qgc: { get: (path: string) => Promise<unknown> } }).Qgc.get(watched), ['/src/bridge/Qgc.ts', path] as const);

const grab = async (page: Page, box: { x: number; y: number; width: number; height: number }) => {
  const [x, y] = [box.x + box.width / 2, box.y + box.height / 2];
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x - box.width / 4, y + box.height / 6, { steps: 12 });
  await page.mouse.up();
};

test('a mouse drag grabs the synthetic view like a map: pulled left and down, it turns right and looks up', async ({ vehicle }) => {
  const box = await syntheticCanvas(vehicle);
  await expect.poll(async () => ((await read(vehicle, 'view.syntheticView')) as { aimable?: boolean }).aimable, { timeout: 30_000 }).toBe(true);
  const before = (await read(vehicle, 'view.syntheticView')) as { pan: number; tilt: number };
  await grab(vehicle, box);
  await expect.poll(async () => ((await read(vehicle, 'view.syntheticView')) as { pan: number }).pan - before.pan, { timeout: 10_000 }).toBeGreaterThan(12);
  const after = (await read(vehicle, 'view.syntheticView')) as { pan: number; tilt: number };
  expect(after.pan - before.pan, 'a quarter of the width spans a quarter of the 70° lens').toBeLessThan(25);
  expect(after.tilt, 'pulling the picture down looks up').toBeGreaterThan(before.tilt + 3);
});

test('with a gimbal, dragging the synthetic view turns the gimbal the same way, by the angle the drag spans', async ({ app }) => {
  await connectMock(app, ['Enable gimbal']);
  const box = await syntheticCanvas(app);
  const yaw = async () => ((await read(app, 'view.gimbal')) as { gimbals?: { bodyYaw: number | null }[] } | null)?.gimbals?.[0]?.bodyYaw ?? null;
  await expect.poll(yaw, { timeout: 30_000 }).not.toBeNull();
  const before = (await yaw()) as number;
  await grab(app, box);
  await expect.poll(async () => ((await yaw()) as number) - before, { timeout: 10_000 }).toBeGreaterThan(12);
  expect(((await yaw()) as number) - before).toBeLessThan(25);
});


test('with a gimbal, a drag on the picked synthetic view turns the view and leaves the real camera where it is', async ({ app }) => {
  await connectMock(app, ['Enable gimbal']);
  await app.evaluate(async (module) => ((await import(module)) as { Qgc: { call: (path: string, ...args: unknown[]) => Promise<unknown> } }).Qgc.call('cameras.add', 'Synthetic view', 'Synthetic View', ''), '/src/bridge/Qgc.ts');
  const box = await syntheticCanvas(app);
  const yaw = async () => ((await read(app, 'view.gimbal')) as { gimbals?: { bodyYaw: number | null }[] } | null)?.gimbals?.[0]?.bodyYaw ?? null;
  const pan = async () => ((await read(app, 'view.syntheticView')) as { pan: number }).pan;
  await expect.poll(yaw, { timeout: 30_000 }).not.toBeNull();
  await expect.poll(async () => ((await read(app, 'view.syntheticView')) as { aimable?: boolean }).aimable, { timeout: 30_000 }).toBe(true);
  const [yawBefore, panBefore] = [(await yaw()) as number, await pan()];
  await grab(app, box);
  await expect.poll(async () => (await pan()) - panBefore, { timeout: 10_000 }).toBeGreaterThan(12);
  expect(Math.abs(((await yaw()) as number) - yawBefore), 'the gimbal is not the picked camera').toBeLessThan(2);
});

test('with a gimbal and the synthetic view standing in for the camera, the tilt slider takes the drag: pulled down, the camera tilts down', async ({ app }) => {
  await connectMock(app, ['Enable camera', 'Enable gimbal']);
  const notice = app.getByRole('status', { name: /showing synthetic view$/ });
  await expect(notice).toBeVisible({ timeout: 30_000 });
  const tilt = app.getByRole('slider', { name: 'Camera tilt' });
  const thumb = tilt.locator('..');
  const box = (await thumb.boundingBox())!;
  const [x, y] = [box.x + box.width / 2, box.y + box.height / 2];
  expect(await thumb.evaluate((it, [atX, atY]) => it.contains(document.elementFromPoint(atX, atY)), [x, y] as const), 'the pointer lands on the thumb').toBe(true);
  const before = Number(await tilt.getAttribute('aria-valuenow'));
  await app.mouse.move(x, y);
  await app.mouse.down();
  await app.mouse.move(x, y + 40, { steps: 10 });
  await app.mouse.up();
  await expect.poll(async () => Number(await tilt.getAttribute('aria-valuenow')), { timeout: 10_000 }).toBeLessThan(before - 10);
  await expect(notice).toHaveCSS('user-select', 'none');
});

const NO_VIDEO = 'No video from MockCam 1 · Stream 1-1 · showing synthetic view';

[
  { screen: 'desktop', viewport: { width: 1280, height: 800 }, hasTouch: false },
  { screen: 'phone held upright', viewport: { width: 390, height: 844 }, hasTouch: true },
  { screen: 'phone held sideways', viewport: { width: 844, height: 390 }, hasTouch: true },
].forEach(({ screen, viewport, hasTouch }) =>
  test.describe(screen, () => {
    test.use({ viewport, hasTouch });

    test(`on a ${screen} the no-video notice steps off the middle of the picture, where the camera points, and keeps its words`, async ({ app }, testInfo) => {
      await connectMock(app, ['Enable camera', 'Enable gimbal'], 'APM ArduCopter Vehicle');
      const notice = app.getByRole('status', { name: NO_VIDEO, exact: true });
      await expect(notice).toBeVisible({ timeout: 30_000 });
      const picture = await syntheticCanvas(app);
      const [x, y] = [picture.x + picture.width / 2, picture.y + picture.height / 2];
      const coversAimPoint = async () => {
        const box = (await notice.boundingBox())!;
        return box.x <= x && x <= box.x + box.width && box.y <= y && y <= box.y + box.height;
      };
      await expect.poll(coversAimPoint, { timeout: 15_000 }).toBe(false);
      await expect(notice).toBeVisible();
      await app.screenshot({ path: testInfo.outputPath(`no-video-notice-${viewport.width}x${viewport.height}.png`) });
    });
  }),
);

type Bottle = { latitude: number; longitude: number; aboveHome: number; landed: boolean };

test('a dropped water bottle falls from the drone and lies on the ground below the gimbal looking down', async ({ app }, testInfo) => {
  await connectMock(app, ['Enable gimbal'], 'APM ArduCopter Vehicle');
  await syntheticCanvas(app);
  await takeOff(app, '10 m');
  await expect(app.getByRole('button', { name: 'Slide to land' })).toBeVisible({ timeout: 60_000 });
  await expect(flightMode(app)).toHaveAccessibleName(/^Guided/, { timeout: 30_000 });
  await app.evaluate(async (module) => ((await import(module)) as { Qgc: { invoke: (path: string, ...args: unknown[]) => Promise<boolean> } }).Qgc.invoke('gimbal.aim', -90, 0), '/src/bridge/Qgc.ts');
  const view = async () => (await read(app, 'view.syntheticView')) as { latitude: number; longitude: number; aboveHome: number; pitch: number; bottles?: Bottle[] };
  await expect.poll(async () => (await view()).pitch, { timeout: 15_000 }).toBeLessThan(-85);
  await expect.poll(async () => (await view()).aboveHome, { timeout: 30_000 }).toBeGreaterThan(9);
  expect((await view()).bottles).toEqual([]);

  await slideAcross(app, app.getByRole('button', { name: 'Slide to drop' }));
  const seen: Bottle[] = [];
  const watched = async () => {
    const bottles = (await view()).bottles ?? [];
    seen.push(...bottles);
    return bottles;
  };
  await expect.poll(async () => (await watched()).some((it) => it.landed || it.aboveHome < 7), { intervals: [25], timeout: 30_000 }).toBe(true);
  await app.screenshot({ path: testInfo.outputPath('bottle-falling.png') });
  await expect.poll(async () => (await watched()).map((it) => it.landed), { intervals: [25], timeout: 30_000 }).toEqual([true]);
  const falling = seen.filter((it) => !it.landed).map((it) => it.aboveHome);
  expect(falling.length, 'it was seen in the air before it landed').toBeGreaterThan(0);
  expect(Math.max(...falling), 'it fell from the drone').toBeGreaterThan(Math.min(...falling));
  await expect(app.getByText('Payload dropped')).toBeVisible();

  const [landed] = (await view()).bottles ?? [];
  const drone = await view();
  expect(Math.abs(landed!.latitude - drone.latitude), 'a bottle dropped in a hover lands under the drone').toBeLessThan(2e-5);
  expect(Math.abs(landed!.longitude - drone.longitude)).toBeLessThan(2e-5);
  expect(landed!.aboveHome, 'on the ground, not in the air').toBeLessThan(drone.aboveHome - 5);
  await app.screenshot({ path: testInfo.outputPath('bottle-on-the-ground.png') });
  expect((await view()).bottles, 'and it stays where it landed').toEqual([landed]);
});

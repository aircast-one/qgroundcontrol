import { type Page } from '@playwright/test';
import { connectMock, expect, flightMode, takeOff, test } from './fixtures';

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
  const notice = app.getByText(/showing synthetic view$/);
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

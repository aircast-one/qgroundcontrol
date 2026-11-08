import type { Locator, Page } from '@playwright/test';
import { connectMock, expect, flightMode, takeOff, test } from './fixtures';

const LONG_PRESS_MS = 400;
const CAMERA_PANEL_OFFSET = 'aircast.fly-overlay-layout.OverlayRigOffset-cameraControl@landscape';

const centre = async (target: Locator): Promise<readonly [number, number]> => {
  const box = await target.boundingBox();
  if (box === null) throw new Error('target is not on screen');
  return [box.x + box.width / 2, box.y + box.height / 2];
};

const editBar = (page: Page): Locator => page.getByRole('button', { name: 'Reset layout' });

const holdUntilEditing = async (page: Page, target: Locator) => {
  const [x, y] = await centre(target);
  await page.mouse.move(x, y);
  await page.mouse.down();
  await expect(editBar(page)).toBeVisible();
  await page.mouse.up();
};

const pastLongPress = (page: Page, since: number) => page.waitForFunction(([from, wait]) => performance.now() - from > wait, [since, LONG_PRESS_MS * 2] as const);

const storedCameraPanel = (page: Page) => page.evaluate((key) => localStorage.getItem(key), CAMERA_PANEL_OFFSET);

const withCamera = async (page: Page) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await connectMock(page, ['Enable camera', 'Enable gimbal']);
  await expect(page.getByRole('button', { name: 'Start recording' })).toBeVisible();
};

const bottomOf = async (target: Locator): Promise<number> => {
  const box = await target.boundingBox();
  if (box === null) throw new Error('target is not on screen');
  return box.y + box.height;
};

const editLayout = async (page: Page) => {
  await page.getByRole('button', { name: 'More', exact: true }).click();
  await page.getByRole('dialog').getByText('Edit layout').click();
  await expect(editBar(page)).toBeVisible();
};

const dragCameraPanel = async (page: Page, dx: number, dy: number) => {
  await editLayout(page);
  const [x, y] = await centre(page.getByRole('button', { name: 'Start recording' }));
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + dx, y + dy, { steps: 20 });
  await page.mouse.up();
  await page.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(page)).toBeHidden();
};

test('holding the shutter starts layout editing without recording, and a tap still records', async ({ app }) => {
  await withCamera(app);
  const shutter = app.getByRole('button', { name: 'Start recording' });
  await holdUntilEditing(app, shutter);
  await app.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(app)).toBeHidden();
  await expect(shutter, 'the release that ended the hold did not press the shutter').toBeVisible();
  await shutter.click();
  await expect(app.getByRole('button', { name: 'Stop recording' }), 'one tap, one toggle: the hold never toggled it').toBeVisible();
});

test('after a hold on an indicator inside the status pill, the keyboard still opens that indicator', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  const battery = vehicle.getByRole('button', { name: /^Battery \d+%/ }).first();
  await holdUntilEditing(vehicle, battery);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(vehicle)).toBeHidden();
  await battery.focus();
  await vehicle.keyboard.press('Enter');
  await expect(vehicle.getByRole('dialog').getByText(/^Battery 1/)).toBeVisible();
});

test('only widgets without layout widgets inside them jiggle, so the status pill does not swing as one slab', async ({ vehicle }) => {
  await editLayout(vehicle);
  const jiggling = () =>
    vehicle.evaluate(() => {
      const targets = document
        .getAnimations()
        .filter((it): it is CSSAnimation => it instanceof CSSAnimation && it.animationName === 'aircast-overlay-jiggle')
        .map((it) => it.effect)
        .filter((it): it is KeyframeEffect => it instanceof KeyframeEffect)
        .map((it) => it.target)
        .filter((it): it is Element => it !== null);
      return { count: targets.length, nested: targets.filter((it) => targets.some((other) => other !== it && other.contains(it))).length };
    });
  await expect.poll(async () => (await jiggling()).count).toBeGreaterThan(1);
  expect((await jiggling()).nested, 'a jiggling widget inside another jiggling widget').toBe(0);
});

test('while editing the layout the video takes no input, and it does again after Done', async ({ app }) => {
  await withCamera(app);
  const [x, y] = await app.evaluate(() => [Math.round(innerWidth * 0.4), Math.round(innerHeight * 0.6)] as const);
  await app.evaluate(([px, py]) => {
    const video = document.elementFromPoint(px, py);
    if (video === null) throw new Error('nothing under the probe point');
    Object.assign(window, { videoPresses: 0 });
    video.addEventListener('pointerdown', () => Object.assign(window, { videoPresses: (window as unknown as { videoPresses: number }).videoPresses + 1 }));
  }, [x, y] as const);
  const presses = () => app.evaluate(() => (window as unknown as { videoPresses: number }).videoPresses);
  await editLayout(app);
  await app.mouse.click(x, y);
  await app.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(app)).toBeHidden();
  expect(await presses(), 'the press made while editing').toBe(0);
  await app.mouse.click(x, y);
  await expect.poll(presses, { message: 'the probe point is the video' }).toBe(1);
});

test('a widget dragged up stops below the status row, also after turning the screen and back', async ({ app }) => {
  await withCamera(app);
  const rowWidth = async () => (await app.locator('[data-status-row]').boundingBox())?.width ?? 0;
  await app.setViewportSize({ width: 412, height: 860 });
  await expect.poll(rowWidth).toBeLessThan(500);
  await app.setViewportSize({ width: 1280, height: 800 });
  await expect.poll(rowWidth).toBeGreaterThan(500);
  await dragCameraPanel(app, 0, -1000);
  const row = await bottomOf(app.locator('[data-status-row]'));
  const zoom = await app.getByRole('button', { name: /^Zoom \d+%$/ }).boundingBox();
  expect(zoom?.y, 'the top of the camera panel').toBeGreaterThanOrEqual(row);
});

test('sideways, an armed vehicle locks layout editing', async ({ vehicle }) => {
  test.setTimeout(120_000);
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  await takeOff(vehicle, '5 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^(Takeoff|Hold)\b/, { timeout: 60_000 });
  const battery = vehicle.getByRole('button', { name: /^Battery \d+%/ }).first();
  const [x, y] = await centre(battery);
  await vehicle.mouse.move(x, y);
  await vehicle.mouse.down();
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold\b/, { timeout: 60_000 });
  await vehicle.mouse.move(x, y + 300, { steps: 4 });
  await vehicle.mouse.up();
  await expect(editBar(vehicle), 'a hold while armed does not edit the layout').toBeHidden();
});

test('holding the flight mode chip, which stays where it is while editing, starts layout editing without opening the mode menu', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  await expect(vehicle.getByRole('button', { name: 'Ready to fly', exact: true }), 'the chip has settled, so nothing moves under the pointer').toBeVisible({ timeout: 60_000 });
  await holdUntilEditing(vehicle, flightMode(vehicle));
  await expect(vehicle.getByRole('menu')).toHaveCount(0);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(vehicle)).toBeHidden();
  await flightMode(vehicle).click();
  await expect(vehicle.getByRole('menu'), 'a tap still opens it').toBeVisible();
});

test('a slow drag of the camera tilt slider tilts the camera and never starts layout editing', async ({ app }) => {
  test.setTimeout(120_000);
  await withCamera(app);
  const tilt = app.getByRole('slider', { name: 'Camera tilt' });
  await expect(tilt).toBeVisible({ timeout: 60_000 });
  const [x, y] = await centre(tilt.locator('..'));
  const before = Number(await tilt.getAttribute('aria-valuenow'));
  await app.mouse.move(x, y);
  await app.mouse.down();
  const pressedAt = await app.evaluate(() => performance.now());
  await app.mouse.move(x, y + 20, { steps: 5 });
  await pastLongPress(app, pressedAt);
  await app.mouse.move(x, y + 40, { steps: 5 });
  await app.mouse.up();
  await expect.poll(async () => Number(await tilt.getAttribute('aria-valuenow'))).toBeLessThan(before - 10);
  await expect(editBar(app), 'the drag held the pointer down past a long press, but it moved').toBeHidden();
});

test('the camera panel drags to a new place that is still there after a reload', async ({ app }) => {
  await withCamera(app);
  const shutter = app.getByRole('button', { name: 'Start recording' });
  const [x, y] = await centre(shutter);
  await dragCameraPanel(app, -300, 120);
  const [movedX, movedY] = await centre(shutter);
  expect(movedX).toBeLessThan(x - 250);
  expect(movedY).toBeGreaterThan(y + 80);
  const stored = await storedCameraPanel(app);
  expect(stored).toMatch(/^-\d+(\.\d+)?,\d+(\.\d+)?$/);
  await app.reload();
  await withCamera(app);
  await expect.poll(async () => (await centre(shutter))[0]).toBeCloseTo(movedX, -1);
  expect((await centre(shutter))[1]).toBeCloseTo(movedY, -1);
  expect(await storedCameraPanel(app)).toBe(stored);
});

test('the camera panel stays on screen in a smaller window and goes back to its place when the window grows back', async ({ app }) => {
  await withCamera(app);
  const shutter = app.getByRole('button', { name: 'Start recording' });
  await dragCameraPanel(app, 0, 600);
  const stored = await storedCameraPanel(app);
  expect(stored, 'dragged down to the bottom edge').toMatch(/^0,\d+(\.\d+)?$/);
  const placed = await centre(shutter);
  await app.setViewportSize({ width: 900, height: 560 });
  await expect.poll(() => bottomOf(shutter), { message: 'pulled up into the smaller window' }).toBeLessThanOrEqual(560);
  expect(await storedCameraPanel(app), 'the pull-back is not saved').toBe(stored);
  await app.setViewportSize({ width: 1280, height: 800 });
  await expect.poll(async () => (await centre(shutter))[1]).toBeCloseTo(placed[1], -1);
  expect(await storedCameraPanel(app)).toBe(stored);
});

test('each camera control hides on its own and the panel closes up around it', async ({ app }) => {
  await withCamera(app);
  await editLayout(app);
  const shutter = app.getByRole('button', { name: 'Start recording' });
  await shutter.locator('xpath=ancestor::div[contains(@class,"relative")][1]').getByRole('button', { name: 'Hide' }).click();
  await app.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(app)).toBeHidden();
  await expect(shutter, 'the hidden shutter').toBeHidden();
  await expect(app.getByRole('button', { name: /^Zoom \d+%$/ }), 'the zoom stays').toBeVisible();
});

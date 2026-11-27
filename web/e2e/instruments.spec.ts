import type { Locator, Page } from '@playwright/test';
import { WHEP_STREAM, addVideoSource, connectMock, expect, flightMode, mapMenuAt, showMapFull, slideAcross, takeOff, test } from './fixtures';

const chooseReadings = async (page: Page) => {
  await page.getByRole('button', { name: 'Menu', exact: true }).click();
  await page.getByRole('dialog').getByText('Choose readings').click();
  return page.getByRole('dialog', { name: 'Choose readings' });
};

const box = async (target: Locator) => {
  const found = await target.boundingBox();
  expect(found, 'drawn on screen').not.toBeNull();
  return found!;
};

test('Choose readings brings back the attitude and compass dials, which follow the aircraft', async ({ vehicle }) => {
  const attitude = vehicle.getByRole('img', { name: /^Attitude, pitch -?\d+°, roll -?\d+°$/ });
  await expect(attitude, 'they start hidden, as an option').toHaveCount(0);
  await (await chooseReadings(vehicle)).getByText('Attitude and compass', { exact: true }).click();
  await vehicle.keyboard.press('Escape');
  await expect(attitude).toBeVisible();
  const compass = vehicle.getByRole('img', { name: /^Heading / }).filter({ visible: true }).last();
  await expect(compass).toBeVisible();
  expect(Math.abs((await box(compass)).y - (await box(attitude)).y), 'the compass sits beside the horizon, like the QGC pair').toBeLessThan(4);
  await expect(attitude, 'level on the ground').toHaveAccessibleName('Attitude, pitch 0°, roll 0°');
  const horizon = () => attitude.locator('g[transform]').first().getAttribute('transform');
  const level = await horizon();
  await takeOff(vehicle, '10 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await showMapFull(vehicle);
  await mapMenuAt(vehicle, 200, -100);
  await vehicle.getByRole('button', { name: 'Go to location' }).click();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to go to location' }));
  await expect(attitude, 'the simulated aircraft dips its nose to fly there, and the dial says so').toHaveAccessibleName(/^Attitude, pitch -[1-9]\d*°, roll -?\d+°$/, { timeout: 30_000 });
  expect(await horizon(), 'and the drawn horizon moved with it').not.toEqual(level);
  await expect(vehicle.getByText(/^Arrived/)).toBeVisible({ timeout: 90_000 });
  await expect(attitude, 'level again in the hover').toHaveAccessibleName('Attitude, pitch 0°, roll 0°', { timeout: 30_000 });
  await (await chooseReadings(vehicle)).getByText('Attitude and compass', { exact: true }).click();
  await vehicle.keyboard.press('Escape');
  await expect(attitude, 'and the same row hides them again').toHaveCount(0);
});

test('held upright, where the dials are never drawn, Choose readings does not offer them', async ({ vehicle }) => {
  await expect((await chooseReadings(vehicle)).getByText('Attitude and compass', { exact: true }), 'offered in landscape').toBeVisible();
  await vehicle.keyboard.press('Escape');
  await vehicle.setViewportSize({ width: 412, height: 860 });
  const upright = await chooseReadings(vehicle);
  await expect(upright.getByPlaceholder(/^Search/), 'the readings are listed').toBeVisible();
  await expect(upright.getByText('Attitude and compass', { exact: true }), 'a box that would show nothing is not offered').toHaveCount(0);
});

const drag = async (page: Page, from: { x: number; y: number }, dx: number, dy: number) => {
  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  await page.mouse.move(from.x + dx, from.y + dy, { steps: 12 });
  await page.mouse.up();
};

const middle = async (target: Locator) => {
  const at = await box(target);
  return { x: at.x + at.width / 2, y: at.y + at.height / 2 };
};

const corner = async (target: Locator) => {
  const at = await box(target);
  return [Math.round(at.x), Math.round(at.y)];
};

test('the small picture in the corner is dragged anywhere on screen, a tap still swaps the map and the camera, and the other picture takes its place', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  const mapButton = vehicle.getByRole('button', { name: 'Map', exact: true });
  await expect(mapButton).toBeVisible({ timeout: 30_000 });
  const home = await box(mapButton);
  await drag(vehicle, await middle(mapButton), 300, -250);
  const moved = await box(mapButton);
  expect([Math.round(moved.x - home.x), Math.round(moved.y - home.y)], 'dropped where the pointer let go, and the drop did not swap the views').toEqual([300, -250]);
  await mapButton.click();
  const video = vehicle.getByRole('button', { name: 'Video', exact: true });
  await expect(video, 'a tap swaps: the map fills the screen and the camera shrinks into the corner').toBeVisible({ timeout: 30_000 });
  const camera = await box(video);
  expect(camera.y, 'the camera picture comes up where the map button was left, not back at the bottom').toBeLessThan(home.y - 200);
  await drag(vehicle, await middle(video), 120, 60);
  const dragged = await box(video);
  expect([Math.round(dragged.x - camera.x), Math.round(dragged.y - camera.y)]).toEqual([120, 60]);
  await video.click();
  await expect(mapButton, 'and a tap on the camera swaps back').toBeVisible();
});

test('the small picture stays where it was left across a restart, and stops at the screen edges below the status row', async ({ vehicle: app }) => {
  await app.emulateMedia({ reducedMotion: 'reduce' });
  const mapButton = app.getByRole('button', { name: 'Map', exact: true });
  await expect(mapButton).toBeVisible({ timeout: 30_000 });
  await drag(app, await middle(mapButton), 300, -250);
  const left = await corner(mapButton);
  await app.reload();
  await connectMock(app);
  await expect(mapButton).toBeVisible({ timeout: 30_000 });
  await expect.poll(() => corner(mapButton), { message: 'the app comes back with the picture where it was dropped' }).toEqual(left);
  await drag(app, await middle(mapButton), 4000, -4000);
  const edge = await box(mapButton);
  const screen = app.viewportSize()!;
  expect(edge.x + edge.width, 'not past the right edge').toBeLessThanOrEqual(screen.width);
  expect(edge.x + edge.width, 'but all the way to it').toBeGreaterThan(screen.width - 40);
  const status = await box(app.getByRole('button', { name: 'Menu', exact: true }));
  expect(edge.y, 'and not up into the status row').toBeGreaterThanOrEqual(status.y + status.height);
  await app.reload();
  await connectMock(app);
  await expect(mapButton).toBeVisible({ timeout: 30_000 });
  await expect.poll(() => corner(mapButton)).toEqual([Math.round(edge.x), Math.round(edge.y)]);
});

test('the small camera picture over the map grows from its corner grip or a pinch, keeps its new size, and a tap still swaps back', async ({ app: vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  await addVideoSource(vehicle, /^WebRTC \(WHEP\)/, WHEP_STREAM);
  await expect.poll(() => vehicle.locator('video').first().evaluate((element: HTMLVideoElement) => element.videoWidth), { timeout: 30_000 }).toBe(640);
  const mapButton = vehicle.getByRole('button', { name: 'Map', exact: true });
  await expect(mapButton).toBeVisible({ timeout: 30_000 });
  await mapButton.click();
  const video = vehicle.getByRole('button', { name: 'Video', exact: true });
  await expect(video).toBeVisible({ timeout: 30_000 });
  await expect.poll(async () => (await video.boundingBox())?.width ?? 0, { message: 'the live camera, not the 48 px synthetic button' }).toBeGreaterThan(100);
  const small = await box(video);
  await video.hover();
  const grip = video.getByTestId('pip-resize');
  await expect(grip, 'the grip shows on hover').toBeVisible();
  const handle = await box(grip);
  await vehicle.mouse.move(handle.x + handle.width / 2, handle.y + handle.height / 2);
  await vehicle.mouse.down();
  await vehicle.mouse.move(handle.x + handle.width / 2 + small.width / 2, handle.y + handle.height / 2, { steps: 12 });
  await vehicle.mouse.up();
  const grown = await box(video);
  expect(Math.round(grown.width / small.width * 10) / 10, 'pulled half its width to the right, it is half again as big').toBe(1.5);
  expect(Math.round(grown.x), 'it grows from its corner, the other corner stays put').toBe(Math.round(small.x));
  await expect(mapButton, 'letting go of the grip did not swap the views').toHaveCount(0);
  const touch = await vehicle.context().newCDPSession(vehicle);
  const [cx, cy] = [grown.x + grown.width / 2, grown.y + grown.height / 2];
  const fingers = (apart: number) => [
    { x: cx - apart, y: cy, id: 1 },
    { x: cx + apart, y: cy, id: 2 },
  ];
  await touch.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: fingers(40) });
  await [32, 26, 20].reduce<Promise<unknown>>((sent, apart) => sent.then(() => touch.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: fingers(apart) })), Promise.resolve());
  await touch.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  const pinched = await box(video);
  expect(Math.round((pinched.width / grown.width) * 10) / 10, 'two fingers pinched to half their spread halve it').toBe(0.5);
  await expect(mapButton, 'the pinch did not swap the views either').toHaveCount(0);
  await video.click();
  await expect(mapButton).toBeVisible();
  await mapButton.click();
  await expect(video).toBeVisible();
  expect(Math.round((await box(video)).width), 'swapped back and forth, it keeps the size it was left at').toBe(Math.round(pinched.width));
});

const liveCameraPip = async (page: Page) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await addVideoSource(page, /^WebRTC \(WHEP\)/, WHEP_STREAM);
  await expect.poll(() => page.locator('video').first().evaluate((element: HTMLVideoElement) => element.videoWidth), { timeout: 30_000 }).toBe(640);
  const mapButton = page.getByRole('button', { name: 'Map', exact: true });
  const video = page.getByRole('button', { name: 'Video', exact: true });
  await expect(mapButton).toBeVisible({ timeout: 30_000 });
  await mapButton.click();
  await expect(video).toBeVisible({ timeout: 30_000 });
  await expect.poll(async () => (await video.boundingBox())?.width ?? 0, { message: 'the live camera, not the 48 px synthetic button' }).toBeGreaterThan(100);
  return { mapButton, video };
};

const growByGrip = async (page: Page, video: Locator) => {
  const small = await box(video);
  await video.hover();
  await drag(page, await middle(video.getByTestId('pip-resize')), small.width / 2, 0);
  return small;
};

test('the size the camera picture was given survives dragging the one-size map button and a restart', async ({ app }) => {
  const { mapButton, video } = await liveCameraPip(app);
  const small = await growByGrip(app, video);
  const grown = await box(video);
  expect(Math.round((grown.width / small.width) * 10) / 10).toBe(1.5);
  await video.click();
  await expect(mapButton).toBeVisible();
  expect((await box(mapButton)).width, 'the map button has one size').toBeLessThan(60);
  await drag(app, await middle(mapButton), 60, -40);
  await mapButton.click();
  await expect(video).toBeVisible();
  await expect.poll(async () => Math.round((await box(video)).width), { message: 'moving the button did not reset the picture to 1x' }).toBe(Math.round(grown.width));
  await app.reload();
  await expect(video, 'the map view comes back with the camera in the corner').toBeVisible({ timeout: 30_000 });
  await expect.poll(async () => Math.round((await box(video)).width), { message: 'and it is still that size after a restart', timeout: 30_000 }).toBe(Math.round(grown.width));
});

test('a camera picture that took the place of a button left at the edge moves with the first pixels of a drag', async ({ app }) => {
  const { mapButton, video } = await liveCameraPip(app);
  await video.click();
  await expect(mapButton).toBeVisible();
  await drag(app, await middle(mapButton), 4000, 0);
  await mapButton.click();
  await expect(video).toBeVisible();
  await expect.poll(async () => (await box(video)).width).toBeGreaterThan(100);
  const screen = app.viewportSize()!;
  const parked = await box(video);
  expect(parked.x + parked.width, 'the wide picture is drawn inside the screen, short of where the button was left').toBeLessThanOrEqual(screen.width);
  await drag(app, await middle(video), -100, 0);
  expect(Math.round((await box(video)).x - parked.x), 'pulled back 100 px, it moves 100 px').toBe(-100);
});

test('the resize grip takes no touches until it is shown, a second finger outside the picture still pinches it, one finger drags it, and the plus and minus keys size it', async ({ app }) => {
  const { mapButton, video } = await liveCameraPip(app);
  const grip = video.getByTestId('pip-resize');
  await app.mouse.move(2, 2);
  const taps = () => grip.evaluate((element) => getComputedStyle(element).pointerEvents);
  await expect.poll(taps, { message: 'unseen, the grip lets a touch through to the picture' }).toBe('none');
  await video.hover();
  await expect.poll(taps, { message: 'shown on hover, it takes the pointer' }).toBe('auto');
  await app.mouse.move(2, 2);
  const before = await box(video);
  const held = { x: before.x + before.width / 2, y: before.y + before.height / 2, id: 1 };
  const outside = (away: number) => ({ x: held.x + before.width + away, y: held.y - before.height - away, id: 2 });
  const touch = await app.context().newCDPSession(app);
  await touch.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [held] });
  await touch.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [held, outside(0)] });
  const spread = Math.hypot(before.width, before.height);
  await [0.5, 1, 1.5].reduce<Promise<unknown>>((sent, more) => sent.then(() => touch.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [held, outside((spread * more * 0.5) / Math.SQRT2)] })), Promise.resolve());
  await touch.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  const pinched = await box(video);
  expect(pinched.width / before.width, 'the second finger landed on the map beside it and spread, and the picture grew').toBeGreaterThan(1.3);
  await expect(mapButton, 'and the pinch did not swap the views').toHaveCount(0);
  const finger = (x: number, y: number) => [{ x, y, id: 1 }];
  const from = await middle(video);
  await touch.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: finger(from.x, from.y) });
  await [30, 60, 90].reduce<Promise<unknown>>((sent, far) => sent.then(() => touch.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: finger(from.x + far, from.y - far) })), Promise.resolve());
  await touch.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  const carried = await box(video);
  expect([Math.round(carried.x - pinched.x), Math.round(carried.y - pinched.y)], 'one finger drags it').toEqual([90, -90]);
  await expect(mapButton, 'and a finger lifted after a drag is not a tap').toHaveCount(0);
  await video.focus();
  await app.keyboard.press('-');
  await expect.poll(async () => (await box(video)).width, { message: 'the minus key makes the focused picture smaller' }).toBeLessThan(carried.width);
  const shrunk = await box(video);
  await app.keyboard.press('+');
  await expect.poll(async () => (await box(video)).width, { message: 'and the plus key bigger' }).toBeGreaterThan(shrunk.width);
  await expect(mapButton, 'neither key swaps the views').toHaveCount(0);
});

import { WHEP_STREAM, addVideoSource, expect, test } from './fixtures';

test('Choose readings brings back the attitude and compass dials, which follow the aircraft', async ({ vehicle }) => {
  const attitude = vehicle.getByRole('img', { name: /^Attitude, pitch -?\d+°, roll -?\d+°$/ });
  await expect(attitude, 'they start hidden, as an option').toHaveCount(0);
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Choose readings').click();
  const chooser = vehicle.getByRole('dialog', { name: 'Choose readings' });
  await chooser.getByText('Attitude and compass', { exact: true }).click();
  await vehicle.keyboard.press('Escape');
  await expect(attitude).toBeVisible();
  const box = (await attitude.boundingBox())!;
  const compass = vehicle.getByRole('img', { name: /^Heading / }).filter({ visible: true }).last();
  await expect(compass).toBeVisible();
  const beside = (await compass.boundingBox())!;
  expect(Math.abs(beside.y - box.y), 'the compass sits beside the horizon, like the QGC pair').toBeLessThan(4);
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Choose readings').click();
  await vehicle.getByRole('dialog', { name: 'Choose readings' }).getByText('Attitude and compass', { exact: true }).click();
  await vehicle.keyboard.press('Escape');
  await expect(attitude, 'and the same row hides them again').toHaveCount(0);
});

test('the small picture in the corner is dragged anywhere on screen, a tap still swaps the map and the camera, and the other picture takes its place', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  const mapButton = vehicle.getByRole('button', { name: 'Map', exact: true });
  await expect(mapButton).toBeVisible({ timeout: 30_000 });
  const home = (await mapButton.boundingBox())!;
  await vehicle.mouse.move(home.x + home.width / 2, home.y + home.height / 2);
  await vehicle.mouse.down();
  await vehicle.mouse.move(home.x + home.width / 2 + 300, home.y + home.height / 2 - 250, { steps: 12 });
  await vehicle.mouse.up();
  const moved = (await mapButton.boundingBox())!;
  expect([Math.round(moved.x - home.x), Math.round(moved.y - home.y)], 'dropped where the pointer let go, and the drop did not swap the views').toEqual([300, -250]);
  await mapButton.click();
  const video = vehicle.getByRole('button', { name: 'Video', exact: true });
  await expect(video, 'a tap swaps: the map fills the screen and the camera shrinks into the corner').toBeVisible({ timeout: 30_000 });
  const camera = (await video.boundingBox())!;
  expect(camera.y, 'the camera picture comes up where the map button was left, not back at the bottom').toBeLessThan(home.y - 200);
  await vehicle.mouse.move(camera.x + camera.width / 2, camera.y + camera.height / 2);
  await vehicle.mouse.down();
  await vehicle.mouse.move(camera.x + camera.width / 2 + 120, camera.y + camera.height / 2 + 60, { steps: 12 });
  await vehicle.mouse.up();
  const dragged = (await video.boundingBox())!;
  expect([Math.round(dragged.x - camera.x), Math.round(dragged.y - camera.y)]).toEqual([120, 60]);
  await video.click();
  await expect(mapButton, 'and a tap on the camera swaps back').toBeVisible();
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
  const small = (await video.boundingBox())!;
  await video.hover();
  const grip = video.getByTestId('pip-resize');
  await expect(grip, 'the grip shows on hover').toBeVisible();
  const handle = (await grip.boundingBox())!;
  await vehicle.mouse.move(handle.x + handle.width / 2, handle.y + handle.height / 2);
  await vehicle.mouse.down();
  await vehicle.mouse.move(handle.x + handle.width / 2 + small.width / 2, handle.y + handle.height / 2, { steps: 12 });
  await vehicle.mouse.up();
  const grown = (await video.boundingBox())!;
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
  const pinched = (await video.boundingBox())!;
  expect(Math.round((pinched.width / grown.width) * 10) / 10, 'two fingers pinched to half their spread halve it').toBe(0.5);
  await expect(mapButton, 'the pinch did not swap the views either').toHaveCount(0);
  await video.click();
  await expect(mapButton).toBeVisible();
  await mapButton.click();
  await expect(video).toBeVisible();
  expect(Math.round((await video.boundingBox())!.width), 'swapped back and forth, it keeps the size it was left at').toBe(Math.round(pinched.width));
});

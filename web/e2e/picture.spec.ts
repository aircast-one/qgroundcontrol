import { type Page } from '@playwright/test';
import { connectMock, expect, flightMode, flyNow, slideAcross, takeOff, telemetry, test } from './fixtures';

test.describe.configure({ timeout: 180_000 });

type Coordinate = { latitude: number; longitude: number };

const read = (page: Page, path: string) => page.evaluate(async ([module, watched]) => ((await import(module)) as { Qgc: { get: (path: string) => Promise<unknown> } }).Qgc.get(watched), ['/src/bridge/Qgc.ts', path] as const);

const aimGimbal = (page: Page, pitch: number, yaw: number) =>
  page.evaluate(async ([module, down, turn]) => ((await import(module)) as { Qgc: { invoke: (path: string, ...args: unknown[]) => Promise<boolean> } }).Qgc.invoke('gimbal.aim', down, turn), ['/src/bridge/Qgc.ts', pitch, yaw] as const);

const gimbalPitch = async (page: Page): Promise<number | null> => ((await read(page, 'view.gimbal')) as { gimbals?: { pitch: number | null }[] } | null)?.gimbals?.[0]?.pitch ?? null;

const metresApart = (a: Coordinate, b: Coordinate): number => {
  const north = (b.latitude - a.latitude) * 111_195;
  const east = (b.longitude - a.longitude) * 111_195 * Math.cos((a.latitude * Math.PI) / 180);
  return Math.hypot(north, east);
};

const shownPoint = async (page: Page): Promise<Coordinate> => {
  const [lat, lon] = await Promise.all([page.getByText(/^Lat: /).textContent(), page.getByText(/^Lon: /).textContent()]);
  return { latitude: Number(lat?.replace('Lat: ', '')), longitude: Number(lon?.replace('Lon: ', '')) };
};

const chosenSpot = (page: Page) => page.locator('svg[aria-label="Chosen spot"]');

const pictureCentre = async (page: Page) => {
  await flyNow(page);
  const canvas = page.locator('canvas:not(.maplibregl-canvas)').filter({ visible: true }).first();
  await expect(canvas).toBeVisible({ timeout: 30_000 });
  const box = (await canvas.boundingBox())!;
  return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
};

test('a right-click on the camera picture flies the aircraft to the spot it shows', async ({ app }) => {
  await connectMock(app, ['Enable gimbal'], 'APM ArduCopter Vehicle');
  await takeOff(app, '20 m');
  await expect(flightMode(app)).toHaveAccessibleName(/ Flying /, { timeout: 60_000 });
  await expect(telemetry(app, 'H')).toHaveText(/^H(19\.[5-9]|20\.[0-4]) m$/, { timeout: 90_000 });
  await aimGimbal(app, -45, 0);
  await expect.poll(() => gimbalPitch(app), { timeout: 30_000 }).toBeLessThan(-44);
  const centre = await pictureCentre(app);
  await app.mouse.click(centre.x, centre.y, { button: 'right' });
  await expect(chosenSpot(app)).toBeVisible();
  await expect(app.getByRole('button', { name: 'Set home here' })).toHaveCount(0);
  await expect(app.getByText('Right-click the picture to fly there')).toHaveCount(0);
  const target = await shownPoint(app);
  const start = (await read(app, 'vehicle.coordinate')) as Coordinate;
  expect(metresApart(start, target), 'looking 45° down from 20 m, the middle of the picture is about 20 m out').toBeGreaterThan(15);
  expect(metresApart(start, target)).toBeLessThan(25);
  await app.getByRole('button', { name: 'Go to location' }).click();
  await expect(app.getByText(/^\d+ m (north|north-east|east|south-east|south|south-west|west|north-west)/)).toBeVisible();
  await slideAcross(app, app.getByRole('button', { name: 'Slide to go to location' }));
  await expect(chosenSpot(app)).toBeHidden();
  await expect(app.getByText(/^Arrived/)).toBeVisible({ timeout: 90_000 });
  const arrived = (await read(app, 'vehicle.coordinate')) as Coordinate;
  expect(metresApart(arrived, target), 'the aircraft holds over the spot the picture showed').toBeLessThan(5);
});

test('a long press on the camera picture opens the fly-there menu, never the browser\'s own menu', async ({ app }) => {
  await connectMock(app, ['Enable gimbal'], 'APM ArduCopter Vehicle');
  await takeOff(app, '20 m');
  await expect(flightMode(app)).toHaveAccessibleName(/ Flying /, { timeout: 60_000 });
  await aimGimbal(app, -45, 0);
  await expect.poll(() => gimbalPitch(app), { timeout: 30_000 }).toBeLessThan(-44);
  const centre = await pictureCentre(app);
  const finger = await app.context().newCDPSession(app);
  await finger.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [centre] });
  await expect(app.getByRole('button', { name: 'Go to location' })).toBeVisible();
  const browserMenu = await app.evaluate(({ x, y }) => document.elementFromPoint(x, y)!.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: x, clientY: y })), centre);
  await finger.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  expect(browserMenu, 'Windows sends the long press as a contextmenu only after the menu is up, so it lands on the menu, not the picture').toBe(false);
  await expect(chosenSpot(app)).toBeVisible();
  await expect(app.getByRole('button', { name: 'Go to location' })).toBeVisible();
});

test('a click on the picture teaches the right-click, and a right-click on the sky says it is not on the ground', async ({ app }) => {
  await connectMock(app, ['Enable gimbal'], 'APM ArduCopter Vehicle');
  await takeOff(app, '10 m');
  await expect(flightMode(app)).toHaveAccessibleName(/ Flying /, { timeout: 60_000 });
  await aimGimbal(app, 20, 0);
  await expect.poll(() => gimbalPitch(app), { timeout: 30_000 }).toBeGreaterThan(19);
  const centre = await pictureCentre(app);
  await app.mouse.click(centre.x, centre.y);
  await expect(app.getByText('Right-click the picture to fly there')).toBeVisible();
  await app.mouse.click(centre.x, centre.y, { button: 'right' });
  await expect(app.getByText(/^That is above the horizon, or farther than /)).toBeVisible({ timeout: 30_000 });
  await expect(app.getByRole('button', { name: 'Go to location' })).toHaveCount(0);
});

test('with the shutter button turned off, the camera joystick still shows and only the shutter goes', async ({ app }) => {
  await connectMock(app, ['Enable gimbal'], 'APM ArduCopter Vehicle');
  await flyNow(app);
  await app.evaluate(async (module) => {
    const { Qgc } = (await import(module)) as { Qgc: { set: (path: string, value: unknown) => Promise<unknown> } };
    await Qgc.set('settings.flyViewSettings.showPhotoVideoControl', false);
    await Qgc.set('settings.gimbalControllerSettings.onScreenStick', true);
  }, '/src/bridge/Qgc.ts');
  await expect(app.getByRole('img', { name: 'Camera joystick' }), 'owel-red-01 had Show the shutter button off, and that hid the whole camera rail with its joystick').toBeVisible({ timeout: 30_000 });
  await expect(app.getByRole('button', { name: 'Start recording' })).toHaveCount(0);
});

test('with camera RC channels set, the camera joystick turns the camera on them and, with no radio to take them back, holds them at centre', async ({ app }) => {
  await connectMock(app, ['Enable gimbal'], 'APM ArduCopter Vehicle');
  await flyNow(app);
  await app.evaluate(async (module) => {
    const { Qgc } = (await import(module)) as { Qgc: { set: (path: string, value: unknown) => Promise<unknown> } };
    await Qgc.set('settings.flyViewSettings.gimbalPanChannel', 13);
    await Qgc.set('settings.flyViewSettings.gimbalTiltChannel', 14);
    await Qgc.set('settings.gimbalControllerSettings.onScreenStick', true);
  }, '/src/bridge/Qgc.ts');
  const held = async () => ((await read(app, 'vehicle.rcChannelOverrides')) as { value?: Record<string, number> }).value ?? {};
  const stick = app.getByRole('img', { name: 'Camera joystick' });
  await expect(stick).toBeVisible({ timeout: 30_000 });
  const pad = (await stick.boundingBox())!;
  await app.mouse.move(pad.x + pad.width / 2, pad.y + pad.height / 2);
  await app.mouse.down();
  await app.mouse.move(pad.x + pad.width + 40, pad.y + pad.height / 2, { steps: 12 });
  await expect.poll(async () => (await held())['13'] ?? 0, { message: 'full right is the pan channel at top speed', timeout: 20_000 }).toBeGreaterThan(1900);
  await app.mouse.up();
  await expect
    .poll(held, { message: 'letting go of overrides on an aircraft with no working receiver trips its radio failsafe a second later', timeout: 20_000 })
    .toEqual({ '13': 1500, '14': 1500 });
});

test('with a gimbal and no camera picture, the gimbal mode still switches', async ({ app }) => {
  await connectMock(app, ['Enable gimbal'], 'APM ArduCopter Vehicle');
  await flyNow(app);
  const mode = app.getByRole('button', { name: /^Video gimbal mode/ });
  await expect(mode).toBeVisible({ timeout: 30_000 });
  await expect(app.getByRole('img', { name: 'Camera joystick' })).toHaveCount(0);
  await expect(async () => {
    if (!(await mode.getAttribute('aria-label'))?.endsWith('Joystick')) await mode.click();
    await expect(mode).toHaveAccessibleName('Video gimbal mode, Joystick', { timeout: 1_000 });
  }).toPass({ timeout: 20_000 });
  await expect(app.getByRole('img', { name: 'Camera joystick' })).toBeVisible();
});

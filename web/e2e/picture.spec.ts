import { type Page } from '@playwright/test';
import { connectMock, expect, flightMode, slideAcross, takeOff, telemetry, test } from './fixtures';

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
  const later = page.getByRole('button', { name: 'Fly now, finish loading in background' });
  if (await later.isVisible()) await later.click();
  const canvas = page.locator('canvas:not(.maplibregl-canvas)').filter({ visible: true }).first();
  await expect(canvas).toBeVisible({ timeout: 30_000 });
  const box = (await canvas.boundingBox())!;
  return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
};

test('a right-click on the camera picture flies the aircraft to the spot it shows', async ({ app }) => {
  await connectMock(app, ['Enable gimbal'], 'APM ArduCopter Vehicle');
  await takeOff(app, '20 m');
  await expect(flightMode(app)).toHaveAccessibleName(/ Flying /, { timeout: 60_000 });
  await expect(telemetry(app, 'H')).toHaveText(/^H(19\.[5-9]|20\.[0-4])m$/, { timeout: 90_000 });
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

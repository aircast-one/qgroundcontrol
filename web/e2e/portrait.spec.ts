import { type Locator, type Page } from '@playwright/test';
import { chooseCameraJoystick, connectMock, expect, flightMode, openApp, slideAcross, takeOff, test } from './fixtures';

test.use({ viewport: { width: 390, height: 844 }, hasTouch: true });
test.describe.configure({ timeout: 120_000 });

test('on a phone the Fly screen runs in portrait and still takes off', async ({ page }) => {
  await openApp(page);
  await connectMock(page);
  await expect(flightMode(page)).toBeVisible();
  await takeOff(page, '5 m');
  await expect(page.getByRole('button', { name: /^(Takeoff|Hold)\b.*Change flight mode$/ })).toBeVisible({ timeout: 60_000 });
});

test('on a phone the take off panel replaces the deck instead of pushing it up over the map', async ({ page }) => {
  await openApp(page);
  await connectMock(page, ['Enable camera']);
  await expect(page.getByRole('button', { name: 'Start recording' })).toBeVisible();
  await page.getByRole('button', { name: /^(Slide to take off|Take off)$/ }).click();
  await expect(page.getByRole('button', { name: /^Slide to take off( anyway)? · / })).toBeVisible();
  await expect(page.getByRole('button', { name: /^(Slide to take off|Take off)$/ }), 'no second Take off stands above the panel').toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Start recording' }), 'the camera buttons leave with the deck rather than landing on the map buttons').toHaveCount(0);
  await page.getByRole('button', { name: 'Cancel' }).click();
  await expect(page.getByRole('button', { name: /^(Slide to take off|Take off)$/ })).toBeVisible();
});

test('on a phone the map credits stay under the map buttons while a guided panel is open', async ({ page }) => {
  await openApp(page);
  await connectMock(page, ['Enable camera']);
  await page.getByRole('button', { name: /^(Slide to take off|Take off)$/ }).click();
  await expect(page.getByRole('button', { name: /^Slide to take off( anyway)? · / })).toBeVisible();
  const layers = page.getByRole('button', { name: 'Map layers' });
  const box = (await layers.boundingBox())!;
  await expect
    .poll(() => page.evaluate(([x, y]) => document.elementFromPoint(x!, y!)?.closest('[aria-label]')?.getAttribute('aria-label') ?? null, [box.x + box.width / 2, box.y + box.height / 2]), {
      message: 'a tap on Map layers reaches Map layers, not the map credits lifted over it',
    })
    .toBe('Map layers');
});

test('on a phone a guided panel or a confirm slide clears the compass and camera chips off the map, so the credits and the mission card are not covered', async ({ page }) => {
  await openApp(page);
  await connectMock(page, ['Enable camera', 'Enable gimbal'], 'APM ArduCopter Vehicle');
  await page.getByRole('button', { name: 'Enlarge the map' }).click();
  const compass = page.getByRole('img', { name: /^Heading \d+°$/ }).first();
  const zoom = page.getByRole('button', { name: /^Zoom \d+%$/ });
  await expect(compass).toBeVisible();
  await expect(zoom).toBeVisible();
  await page.getByRole('button', { name: /^(Slide to take off|Take off)$/ }).click();
  await expect(page.getByRole('button', { name: /^Slide to take off( anyway)? · / })).toBeVisible();
  await expect(compass, 'the compass stood on the mission card').toBeHidden();
  await expect(zoom, 'the zoom chip sat under the map credits').toBeHidden();
  await page.getByRole('button', { name: 'Cancel' }).click();
  await expect(zoom).toBeVisible();
  await takeOff(page, '5 m');
  await expect(page.getByRole('button', { name: /Flying Change flight mode$|^(Takeoff|Hold|Guided)\b.*Change flight mode$/ })).toBeVisible({ timeout: 60_000 });
  await page.getByRole('button', { name: 'Return', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Slide to return' })).toBeVisible();
  await expect(zoom, 'a confirm slide takes the deck like a guided panel does').toBeHidden();
  await expect(compass).toBeHidden();
  await page.getByRole('button', { name: 'Cancel' }).click();
  await expect(compass).toBeVisible();
});

const gimbalPitch = (page: Page) =>
  page.evaluate(
    async (module) => (((await import(module)) as { Qgc: { get: (path: string) => Promise<unknown> } }).Qgc.get('view.gimbal') as Promise<{ gimbals?: { pitch: number | null }[] } | null>).then((it) => it?.gimbals?.[0]?.pitch ?? null),
    '/src/bridge/Qgc.ts',
  );

test('held upright, a drag up on the camera strip tilts the gimbal down without swapping to the map, and the tilt control on the strip looks straight down', async ({ page }) => {
  await openApp(page);
  await connectMock(page, ['Enable camera', 'Enable gimbal'], 'APM ArduCopter Vehicle');
  await takeOff(page, '5 m');
  await expect(page.getByRole('status', { name: /showing synthetic view$/ })).toBeVisible({ timeout: 30_000 });
  await expect.poll(() => gimbalPitch(page), { timeout: 60_000 }).not.toBeNull();
  const picture = page.locator('canvas:not(.maplibregl-canvas)').filter({ visible: true }).first();
  const strip = (await picture.boundingBox())!;
  const enlargeMap = page.getByRole('button', { name: 'Enlarge the map' });
  await expect(enlargeMap).toBeVisible();
  const before = (await gimbalPitch(page))!;
  const [x, y] = [strip.x + strip.width / 2, strip.y + strip.height - 24];
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x, y - strip.height / 3, { steps: 12 });
  await page.mouse.up();
  await expect.poll(async () => (await gimbalPitch(page))! - before, { timeout: 15_000 }).toBeLessThan(-8);
  await expect(enlargeMap, 'the camera stays above the map').toBeVisible();
  expect(await picture.boundingBox()).toEqual(strip);

  await chooseCameraJoystick(page);
  const opener = page.getByRole('button', { name: /^Tilt -?\d+°$/ });
  const at = (await opener.boundingBox())!;
  expect(at.y, 'on the camera strip').toBeGreaterThanOrEqual(strip.y);
  expect(at.y + at.height, 'on the camera strip').toBeLessThanOrEqual(strip.y + strip.height);
  expect(at.height, 'a finger-sized target').toBeGreaterThanOrEqual(44);
  const [cx, cy] = [at.x + at.width / 2, at.y + at.height / 2];
  expect(await opener.evaluate((it, [px, py]) => it.contains(document.elementFromPoint(px, py)), [cx, cy] as const), 'nothing covers it').toBe(true);
  await page.mouse.click(cx, cy);
  const stick = page.getByRole('img', { name: 'Camera joystick' });
  await expect(stick, 'the mock gimbal has a yaw axis, so it opens a joystick').toBeVisible();
  const pad = (await stick.boundingBox())!;
  await page.mouse.move(pad.x + pad.width / 2, pad.y + pad.height / 2);
  await page.mouse.down();
  await page.mouse.move(pad.x + pad.width / 2, pad.y + pad.height + 40, { steps: 12 });
  await expect.poll(() => gimbalPitch(page), { timeout: 20_000 }).toBeLessThan(-88);
  await page.mouse.up();
});

const editBar = (page: Page) => page.getByRole('button', { name: 'Layout options' });

const battery = (page: Page) => page.getByRole('button', { name: /^Battery \d+%/ }).first();

const pressAndHold = async (page: Page, target: Locator, held: () => Promise<unknown>) => {
  const box = (await target.boundingBox())!;
  const [x, y] = [box.x + box.width / 2, box.y + box.height / 2];
  await page.mouse.move(x, y);
  await page.mouse.down();
  await held();
  await page.mouse.move(x, y + box.height * 6, { steps: 4 });
  await page.mouse.up();
};

const pressable = (page: Page, name: string) =>
  page.getByRole('button', { name, exact: true }).evaluate((button) => {
    const box = button.getBoundingClientRect();
    return button.contains(document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2));
  });

test('held upright, a hold in flight edits the layout, and Return and Land stay pressable beside the edit bar', async ({ page }) => {
  test.setTimeout(240_000);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await openApp(page);
  await connectMock(page);
  await takeOff(page, '5 m');
  await expect(flightMode(page)).toHaveAccessibleName(/^Hold\b/, { timeout: 60_000 });
  await pressAndHold(page, battery(page), () => expect(editBar(page)).toBeVisible());
  expect(await pressable(page, 'Return'), 'Return is not under the edit bar').toBe(true);
  expect(await pressable(page, 'Slide to land'), 'Land is not under the edit bar').toBe(true);
  await page.getByRole('button', { name: 'Return', exact: true }).click();
  await slideAcross(page, page.getByRole('button', { name: 'Slide to return' }));
  await expect(flightMode(page)).toHaveAccessibleName(/^Return/);
});

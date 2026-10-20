import { type Page } from '@playwright/test';
import { connectMock, expect, flightMode, openApp, takeOff, test } from './fixtures';

test.use({ viewport: { width: 390, height: 844 }, hasTouch: true });
test.describe.configure({ timeout: 120_000 });

test('on a phone the Fly screen runs in portrait and still takes off', async ({ page }) => {
  await openApp(page);
  await connectMock(page);
  await expect(flightMode(page)).toBeVisible();
  await takeOff(page, '5 m');
  await expect(page.getByRole('button', { name: /^(Takeoff|Hold)\b.*Change flight mode$/ })).toBeVisible({ timeout: 60_000 });
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
  await expect(page.getByText(/showing synthetic view$/)).toBeVisible({ timeout: 30_000 });
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

  const opener = page.getByRole('button', { name: /^Tilt -?\d+°$/ });
  const at = (await opener.boundingBox())!;
  expect(at.y, 'on the camera strip').toBeGreaterThanOrEqual(strip.y);
  expect(at.y + at.height, 'on the camera strip').toBeLessThanOrEqual(strip.y + strip.height);
  expect(at.height, 'a finger-sized target').toBeGreaterThanOrEqual(44);
  const [cx, cy] = [at.x + at.width / 2, at.y + at.height / 2];
  expect(await opener.evaluate((it, [px, py]) => it.contains(document.elementFromPoint(px, py)), [cx, cy] as const), 'nothing covers it').toBe(true);
  await page.mouse.click(cx, cy);
  const slider = page.getByRole('slider', { name: 'Camera tilt' });
  await expect(slider).toBeVisible();
  const track = (await slider.locator('xpath=ancestor::*[@data-gesture][1]').boundingBox())!;
  const thumb = (await slider.locator('..').boundingBox())!;
  await page.mouse.move(thumb.x + thumb.width / 2, thumb.y + thumb.height / 2);
  await page.mouse.down();
  await page.mouse.move(thumb.x + thumb.width / 2, track.y + track.height + 40, { steps: 12 });
  await page.mouse.up();
  await expect.poll(() => gimbalPitch(page), { timeout: 20_000 }).toBeLessThan(-88);
});

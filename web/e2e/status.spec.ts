import { type Locator, type Page } from '@playwright/test';
import { connectMock, expect, flightMode, slideAcross, test, vehicleStatus } from './fixtures';

test('the status sheet arms the aircraft without starting layout editing', async ({ vehicle }) => {
  const sheet = await vehicleStatus(vehicle);
  await expect(sheet.getByText('Sensors', { exact: true }), 'the sheet has its sensors, so it stops growing around the slider').toBeVisible();
  await expect(sheet.getByText('Ground control comm loss failsafe'), 'the sheet has its failsafe settings, so it stops growing around the slider').toBeVisible();
  await slideAcross(vehicle, sheet.getByRole('button', { name: 'Slide to arm' }));
  await expect(vehicle.getByText('Arm the vehicle.')).toBeVisible();
  await expect(vehicle.getByRole('button', { name: 'Reset layout' })).toBeHidden();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to arm' }));
  await expect(vehicle.getByRole('button', { name: /Change flight mode$/ }).first()).toContainText('Armed');
});

test('the status sheet scrolls as one page, with no scroll box nested inside it', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  const sheet = await vehicleStatus(app);
  await expect(sheet.getByText('Failsafe options bitmask')).toBeVisible();
  const nested = await sheet.evaluate((root) => [...root.querySelectorAll('*')].filter((it) => it.scrollHeight > it.clientHeight + 1 && ['auto', 'scroll'].includes(getComputedStyle(it).overflowY)).length);
  expect(nested, 'the failsafe settings scroll inside the sheet instead of with it').toBe(0);
});

test('the status sheet lists sensor health and opens the message log', async ({ vehicle }) => {
  const sheet = await vehicleStatus(vehicle);
  await sheet.getByRole('button', { name: 'Show 1 more' }).click();
  await expect(sheet.getByRole('button', { name: 'GPS Normal' }), 'the simulated vehicle reported its GPS switched off over a 12-satellite fix').toBeVisible();
  await sheet.getByRole('button', { name: 'Messages', exact: true }).click();
  const log = vehicle.getByRole('dialog').filter({ hasText: 'messages from the vehicle' });
  await expect(log).toBeVisible();
  await log.getByRole('button', { name: 'Close' }).click();
  await expect(log).toBeHidden();
});

test('status text from the vehicle fills the message log and clear empties it', async ({ app }) => {
  await connectMock(app, ['Send status text + voice']);
  const sheet = await vehicleStatus(app);
  await sheet.getByRole('button', { name: 'Messages', exact: true }).click();
  const log = app.getByRole('dialog').filter({ hasText: /errors? · \d+ warnings?/ });
  await expect(log.getByText('Status text critical')).toBeVisible();
  await expect(log.getByText('Status text info')).toBeVisible();
  await log.getByRole('button', { name: 'Clear' }).click();
  await expect(log).toBeHidden();
  const again = await vehicleStatus(app);
  await again.getByRole('button', { name: 'Messages', exact: true }).click();
  await expect(app.getByText('The vehicle has not said anything yet.')).toBeVisible();
});

test('at desktop width the status pill shows battery time, current draw and satellites', async ({ vehicle }) => {
  await expect(vehicle.getByRole('button', { name: /^\d+:\d\d flight time left$/ })).toBeVisible();
  await expect(vehicle.getByRole('button', { name: /^[\d.]+A current draw$/ })).toBeVisible();
  await expect(vehicle.getByText(/^\d+$/).filter({ hasText: '12' }).first()).toBeVisible();
});

test('all status lists the gimbal and Tilt 90 points it as far down as it goes', async ({ app }) => {
  await connectMock(app, ['Enable gimbal']);
  await app.getByRole('button', { name: 'All status' }).click();
  const gimbal = app.getByRole('button', { name: /· P: -?[\d.]+ ·/ });
  await expect(gimbal).toBeVisible({ timeout: 30_000 });
  const left = async (locator: Locator) => (await locator.boundingBox())?.x ?? Number.NaN;
  expect(Math.abs((await left(app.getByText(/^ESC \d+ OK$/))) - (await left(gimbal)))).toBeLessThan(2);
  await gimbal.click();
  await app.getByRole('button', { name: 'Tilt 90' }).click();
  await expect(app.getByRole('button', { name: /· P: -90\.0 ·/ })).toBeVisible({ timeout: 30_000 });
});

const editLayout = async (page: Page) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.getByRole('button', { name: 'More', exact: true }).click();
  await page.getByRole('dialog').getByText('Edit layout').click();
  await expect(page.getByRole('button', { name: 'Reset layout' })).toBeVisible();
};

const fitsAndKeepsTheFlightMode = async (page: Page) => {
  await editLayout(page);
  const scrollsInside = () =>
    page.getByRole('timer', { name: 'Flight time' }).evaluate((node, width) => {
      const scroller = (it: Element | null): Element | null => (it === null || getComputedStyle(it).overflowX === 'auto' ? it : scroller(it.parentElement));
      const found = scroller(node);
      return found !== null && found.getBoundingClientRect().right <= width && found.scrollWidth > found.clientWidth;
    }, page.viewportSize()?.width ?? 0);
  await expect.poll(scrollsInside, { message: 'the strip ends inside the screen and scrolls to the indicators past its end' }).toBe(true);
  const cut = await flightMode(page).evaluate((chip) => [chip, ...chip.querySelectorAll('*')].filter((it) => getComputedStyle(it).textOverflow === 'ellipsis' && (it.clientWidth === 0 || it.scrollWidth > it.clientWidth + 1)).length);
  expect(cut, 'the flight mode is squeezed out of its chip').toBe(0);
};

test('while editing the layout the status strip scrolls inside the screen and the flight mode keeps its name', async ({ vehicle }) => {
  await fitsAndKeepsTheFlightMode(vehicle);
});

test.describe('on a phone', () => {
  test.use({ viewport: { width: 412, height: 860 } });

  test('while editing the layout the status strip scrolls inside the screen and the flight mode keeps its name', async ({ vehicle }) => {
    await fitsAndKeepsTheFlightMode(vehicle);
  });
});

test('› moves an indicator past the next one on screen, and only indicators on screen can be moved', async ({ vehicle }) => {
  await editLayout(vehicle);
  const timer = vehicle.getByRole('timer', { name: 'Flight time' });
  const gps = vehicle.getByRole('button', { name: /sats$/ });
  const left = async (locator: Locator) => (await locator.boundingBox())?.x ?? Number.NaN;
  expect(await left(timer)).toBeLessThan(await left(gps));
  const earlier = vehicle.getByRole('button', { name: '‹', exact: true });
  const later = vehicle.getByRole('button', { name: '›', exact: true });
  await expect(earlier.first(), 'the first indicator has nowhere further back to go').toBeDisabled();
  await expect(later.last(), 'the last indicator has nowhere further on to go').toBeDisabled();
  const pairs = await Promise.all([earlier.all(), later.all()]);
  expect(pairs[0].length, 'every indicator has both arrows').toBe(pairs[1].length);
  const boxes = await Promise.all(pairs.map((arrows) => Promise.all(arrows.map((it) => it.boundingBox()))));
  const between = boxes[0].map((back, at) => (boxes[1][at]?.x ?? 0) - ((back?.x ?? 0) + (back?.width ?? 0)));
  expect(between.filter((room) => room < 8), 'arrows with no indicator between them').toEqual([]);
  const covered = await vehicle.evaluate(() => {
    const rects = (match: (it: HTMLButtonElement) => boolean) => [...document.querySelectorAll('button')].filter(match).map((it) => it.getBoundingClientRect());
    const badges = rects((it) => it.getAttribute('aria-label') === 'Hide');
    return rects((it) => it.textContent === '‹' || it.textContent === '›').filter((arrow) => badges.some((badge) => arrow.left < badge.right && badge.left < arrow.right && arrow.top < badge.bottom && badge.top < arrow.bottom)).length;
  });
  expect(covered, 'arrows under a hide badge').toBe(0);
  await timer.locator('xpath=../..').getByRole('button', { name: '›', exact: true }).click();
  await expect.poll(async () => (await left(timer)) - (await left(gps))).toBeGreaterThan(0);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await expect(timer, 'the flight time moved rather than hid').toBeVisible();
  expect(await left(timer)).toBeGreaterThan(await left(gps));
});

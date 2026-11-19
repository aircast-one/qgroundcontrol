import type { Locator, Page } from '@playwright/test';
import { connectMock, dropFromStrip, expect, flightMode, showMapFull, slideAcross, takeOff, telemetry, test } from './fixtures';

test('on a desktop the Fly menu holds where to go, what the aircraft can do and how the screen is laid out', async ({ vehicle }) => {
  await expect(vehicle.getByRole('navigation')).toHaveCount(0);
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  const sheet = vehicle.getByRole('dialog');
  const section = (name: string) => sheet.getByRole('region', { name });
  await expect(sheet.getByRole('button', { name: 'Plan', exact: true })).toBeVisible();
  await expect(sheet.getByRole('button', { name: 'Settings', exact: true })).toBeVisible();
  await expect(section('Aircraft').getByRole('button', { name: 'All status', exact: true })).toBeVisible();
  await expect(section('Aircraft').getByRole('button', { name: 'Payload', exact: true })).toBeVisible();
  await expect(section('Screen').getByRole('button', { name: 'Choose readings', exact: true })).toBeVisible();
  await expect(section('Screen').getByRole('button', { name: 'Edit layout', exact: true })).toBeVisible();
});

test('on the ground Force arm sits in the menu in the same ink as Arm and still asks for a slide', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  const aircraft = vehicle.getByRole('dialog').getByRole('region', { name: 'Aircraft' });
  const ink = (name: string) => aircraft.getByRole('button', { name, exact: true }).evaluate((tile) => getComputedStyle(tile).color);
  await expect(aircraft.getByRole('button', { name: 'Force arm', exact: true })).toBeVisible();
  expect(await ink('Force arm'), 'bypassing the arming checks does not shout louder than arming').toBe(await ink('Arm'));
  await aircraft.getByRole('button', { name: 'Force arm', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: 'Slide to force arm' })).toBeVisible();
});

test('edit layout from more actions starts and ends layout editing', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Edit layout').click();
  await expect(vehicle.getByRole('button', { name: 'Layout options' })).toBeVisible();
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await expect(vehicle.getByRole('button', { name: 'Layout options' })).toBeHidden();
});

test('the layout edit controls replace the menu, leave the map and video alone, and the size grows the on-screen readouts', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  const map = vehicle.getByRole('button', { name: 'Map', exact: true, includeHidden: true });
  const mapPip = (await map.boundingBox())!;
  const readout = telemetry(vehicle, 'H').filter({ visible: true }).first().locator('span').last();
  const height = () => readout.evaluate((it) => it.getBoundingClientRect().height);
  const before = await height();
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Edit layout').click();
  const options = vehicle.getByRole('button', { name: 'Layout options' });
  await expect(options).toBeVisible();
  await expect(vehicle.getByRole('button', { name: 'Done' })).toBeVisible();
  await expect(vehicle.getByRole('button', { name: 'Menu', exact: true })).toBeHidden();
  expect(await map.boundingBox(), 'nothing moves to make room for the edit controls').toEqual(mapPip);

  await vehicle.mouse.click(mapPip.x + mapPip.width / 2, mapPip.y + mapPip.height / 2);
  await options.click();
  const size = vehicle.getByRole('menuitem', { name: /^Readout size: / });
  await expect(size).toHaveText('Readout size: Default');
  await size.click();
  await size.click();
  await expect(size).toHaveText('Readout size: Medium');
  await expect.poll(height).toBeCloseTo(before * 1.25, 0);
  await vehicle.keyboard.press('Escape');
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await expect(vehicle.getByRole('button', { name: 'Menu', exact: true })).toBeVisible();
  expect(await map.boundingBox(), 'the map stayed in its corner').toEqual(mapPip);
});

test('reset layout in the layout options asks for a second tap before it resets', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Edit layout').click();
  await vehicle.getByRole('button', { name: 'Layout options' }).click();
  await vehicle.getByRole('menuitem', { name: 'Reset positions and hidden items' }).click();
  await vehicle.getByRole('menuitem', { name: /again to reset$/ }).click();
  await expect(vehicle.getByRole('menu')).toBeHidden();
  await expect(vehicle.getByRole('button', { name: 'Layout options' }), 'still editing after the reset').toBeVisible();
});

test('choose readings opens a chooser of the same name whose groups and readings read as words', async ({ app }) => {
  await connectMock(app, ['Enable gimbal']);
  await app.getByRole('button', { name: 'Menu', exact: true }).click();
  await app.getByRole('dialog').getByText('All status').click();
  await expect(app.getByRole('button', { name: /· P: -?[\d.]+ ·/ }), 'the gimbal has been discovered').toBeVisible({ timeout: 30_000 });
  await expect(app.getByRole('dialog').getByRole('button', { name: /×2/ }), 'both packs have reported').toBeVisible();
  await app.keyboard.press('Escape');
  await expect(app.getByRole('dialog')).toHaveCount(0);
  await app.getByRole('button', { name: 'Menu', exact: true }).click();
  await app.getByRole('dialog').getByText('Choose readings').click();
  const chooser = app.getByRole('dialog', { name: 'Choose readings' });
  await expect(chooser.getByText('Estimator status', { exact: true })).toBeVisible();
  await expect(chooser.getByText('Gimbal', { exact: true }), 'one gimbal is not named by its manager and device ids').toBeVisible();
  await expect(chooser.getByText('Battery 1', { exact: true })).toBeVisible();
  await expect(chooser.getByText(/^(EstimatorStatus|Gimbal\d+|Battery\d|managerCompid|runtime)$/)).toHaveCount(0);
  await expect(chooser.getByText('Time remaining', { exact: true }), 'one Time remaining row per pack').toHaveCount(2);
});

test('map layers switches the base map', async ({ vehicle: app }) => {
  await showMapFull(app);
  await app.getByRole('button', { name: 'Map layers' }).click();
  const layers = app.getByRole('dialog').filter({ hasText: 'Map layers' });
  await layers.getByRole('button', { name: 'Road', exact: true }).click();
  await expect(layers.getByRole('button', { name: 'Road', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await app.keyboard.press('Escape');
  await expect(app.getByText('Bing Road', { exact: true })).toBeVisible();
});

test('the payload sheet offers drop and grab and reports what the aircraft answered', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Payload').click();
  await expect(vehicle.getByText('No payload command sent yet')).toBeVisible();
  await vehicle.getByRole('dialog').getByRole('button', { name: 'Grab', exact: true }).click();
  await expect(vehicle.getByText(/^Grabbed \d+ s ago$/)).toBeVisible();
  await expect(vehicle.getByRole('dialog').getByRole('button', { name: 'Slide to drop' })).toBeVisible();
});

test('in flight the payload strip is the only drop: the deck offers none, and the strip drops once armed', async ({ vehicle }) => {
  test.setTimeout(120_000);
  await expect(vehicle.getByRole('button', { name: 'Take off' })).toBeVisible();
  await takeOff(vehicle, '5 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await expect(vehicle.getByRole('group', { name: 'Flight actions' }).getByRole('button', { name: 'Return' })).toBeVisible();
  await expect(vehicle.getByRole('button', { name: 'Slide to drop' })).toBeHidden();
  await dropFromStrip(vehicle);
  await expect(vehicle.getByText('Payload dropped')).toBeVisible();
});

type Target = { name: string; left: number; top: number; right: number; bottom: number; clipped: boolean };

const FLIGHT_ACTIONS = [/pause/i, /return/i, /land/i, /^Kill motors$/];

const flyWithFourBays = async (app: Page): Promise<Locator> => {
  await app.evaluate(async (module) => {
    const { Qgc } = (await import(module)) as { Qgc: { set: (path: string, value: unknown) => Promise<unknown> } };
    await Qgc.set('settings.flyViewSettings.payloadBays', 4);
  }, '/src/bridge/Qgc.ts');
  await connectMock(app, ['Enable camera', 'Enable gimbal'], 'APM ArduCopter Vehicle');
  await app.getByRole('button', { name: 'Take off', exact: true }).click();
  await app.getByRole('button', { name: '5 m', exact: true }).click();
  await slideAcross(app, app.getByRole('button', { name: /^Slide to take off anyway · / }));
  const actions = app.getByRole('group', { name: 'Flight actions' });
  await Promise.all(FLIGHT_ACTIONS.map((name) => expect(actions.getByRole('button', { name })).toBeVisible({ timeout: 60_000 })));
  return actions;
};

const targets = (actions: Locator): Promise<readonly Target[]> =>
  actions.getByRole('button').evaluateAll((buttons) =>
    buttons.filter((it) => it instanceof HTMLElement).map((button) => {
      const box = button.getBoundingClientRect();
      const blocks = [button, ...button.querySelectorAll('*')].filter((it) => it instanceof HTMLElement && getComputedStyle(it).display !== 'inline');
      return {
        name: button.getAttribute('aria-label') ?? button.querySelector('[aria-label]')?.getAttribute('aria-label') ?? button.innerText,
        left: box.left,
        top: box.top,
        right: box.right,
        bottom: box.bottom,
        clipped: blocks.some((it) => it.scrollWidth > it.clientWidth || it.scrollHeight > it.clientHeight),
      };
    }),
  );

const overlaps = (a: Target, b: Target): boolean => a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom;

const expectFullSizeTargets = (shown: readonly Target[]) => {
  expect(shown.map((it) => it.name)).toEqual(expect.arrayContaining(FLIGHT_ACTIONS.map((name) => expect.stringMatching(name))));
  expect(shown.filter((it) => /drop/i.test(it.name)), 'the payload strip is the only drop').toEqual([]);
  expect(shown.filter((it) => it.right - it.left < 44 || it.bottom - it.top < 44 || it.clipped)).toEqual([]);
  expect(shown.filter((it, at) => shown.some((other, index) => index !== at && overlaps(it, other)))).toEqual([]);
};

[
  { width: 390, height: 844 },
  { width: 360, height: 740 },
].map((viewport) =>
  test.describe(`on a ${viewport.width}x${viewport.height} phone`, () => {
    test.use({ viewport, hasTouch: true });
    test.describe.configure({ timeout: 120_000 });

    test('every flight action on the deck is a full-size target with its whole label while flying with four bays', async ({ app }) => {
      expectFullSizeTargets(await targets(await flyWithFourBays(app)));
    });
  }),
);

test.describe('on a phone held sideways', () => {
  test.use({ viewport: { width: 844, height: 390 }, hasTouch: true });
  test.describe.configure({ timeout: 120_000 });

  test('the deck starts below the top bar and every flight action is a full-size target that overlaps nothing', async ({ app }) => {
    const shown = await targets(await flyWithFourBays(app));
    expectFullSizeTargets(shown);
    const menu = await app.getByRole('button', { name: 'Menu', exact: true }).boundingBox();
    expect(Math.min(...shown.map((it) => it.top))).toBeGreaterThanOrEqual((menu?.y ?? 0) + (menu?.height ?? 0));
    expect(Math.max(...shown.map((it) => it.bottom))).toBeLessThanOrEqual(390);
  });
});

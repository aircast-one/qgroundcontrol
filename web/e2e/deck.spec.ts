import type { Locator, Page } from '@playwright/test';
import { connectMock, dropFromStrip, expect, flightMode, showMapFull, slideAcross, takeOff, telemetry, test } from './fixtures';

test('more actions lists every deck command', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'More', exact: true }).click();
  const sheet = vehicle.getByRole('dialog').filter({ hasText: 'More actions' });
  await expect(sheet).toBeVisible();
  await expect(sheet.getByText('Choose readings')).toBeVisible();
  await expect(sheet.getByText('Edit layout')).toBeVisible();
  await expect(sheet.getByText('Payload')).toBeVisible();
});

test('edit layout from more actions starts and ends layout editing', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'More', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Edit layout').click();
  await expect(vehicle.getByRole('button', { name: 'Reset layout' })).toBeVisible();
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await expect(vehicle.getByRole('button', { name: 'Reset layout' })).toBeHidden();
});

const paintedOnTopAcross = (control: Locator): Promise<readonly boolean[]> =>
  control.evaluate((it) => {
    const everything = Object.assign(document.createElement('style'), { textContent: '* { pointer-events: auto !important; }' });
    document.head.append(everything);
    const box = it.getBoundingClientRect();
    const y = box.top + box.height / 2;
    const onTop = [box.left + 4, box.left + box.width / 2, box.right - 4].map((x) => it.contains(document.elementFromPoint(x, y)));
    everything.remove();
    return onTop;
  });

test('the layout edit bar sits above every fly overlay, leaves the map and video alone, and its size grows the on-screen readouts', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  const mapPip = (await vehicle.getByRole('region', { name: 'Map' }).boundingBox())!;
  const readout = telemetry(vehicle, 'H').filter({ visible: true }).first().locator('span').last();
  const height = () => readout.evaluate((it) => it.getBoundingClientRect().height);
  const before = await height();
  await vehicle.getByRole('button', { name: 'More', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Edit layout').click();
  const size = vehicle.getByRole('button', { name: /^Size: / });
  const controls = [size, vehicle.getByRole('button', { name: 'Reset layout' }), vehicle.getByRole('button', { name: 'Done' })];
  await Promise.all(controls.map((it) => expect(it).toBeVisible()));
  expect(await Promise.all(controls.map(paintedOnTopAcross))).toEqual(controls.map(() => [true, true, true]));

  await vehicle.mouse.click(mapPip.x + mapPip.width / 2, mapPip.y + mapPip.height / 2);
  await expect(size).toHaveText('Size: Default');
  await size.click();
  await size.click();
  await expect(size).toHaveText('Size: Medium');
  await expect.poll(height).toBeCloseTo(before * 1.25, 0);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  expect(await vehicle.getByRole('region', { name: 'Map' }).boundingBox(), 'the map stayed in its corner').toEqual(mapPip);
});

test('choose readings opens the readings chooser', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'More', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Choose readings').click();
  await expect(vehicle.getByRole('dialog').filter({ hasNotText: 'More actions' })).toBeVisible();
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
  await vehicle.getByRole('button', { name: 'More', exact: true }).click();
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

const FLIGHT_ACTIONS = [/pause/i, /return/i, /land/i, /^More$/];

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

import type { Locator, Page } from '@playwright/test';
import { connectMock, dropFromStrip, expect, flightMode, showMapFull, slideAcross, takeOff, telemetry, test } from './fixtures';

test('on a desktop the Fly menu leads with Plan, keeps the aircraft and its setup together, and the app and its settings together', async ({ vehicle }) => {
  await expect(vehicle.getByRole('navigation')).toHaveCount(0);
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  const sheet = vehicle.getByRole('dialog');
  const section = (name: string) => sheet.getByRole('region', { name });
  await expect(sheet.getByRole('button', { name: 'Plan', exact: true }), 'open, plan, fly: Plan stands on its own above the groups').toBeVisible();
  await expect(sheet.getByRole('region').getByRole('button', { name: 'Plan', exact: true })).toHaveCount(0);
  const aircraft = ['All status', 'Payload', 'Calibrate', 'Setup', 'Analyze'];
  await Promise.all(aircraft.map((name) => expect(section('Aircraft').getByRole('button', { name, exact: true })).toBeVisible()));
  const screen = ['Choose readings', 'Edit layout', 'Settings'];
  await Promise.all(screen.map((name) => expect(section('Screen').getByRole('button', { name, exact: true })).toBeVisible()));
});

test('Plan in the Fly menu has its own route glyph, so it no longer looks like the map button beside it', async ({ vehicle }) => {
  const glyph = (button: Locator) => button.locator('svg path').first().getAttribute('d');
  const mapGlyph = await glyph(vehicle.getByRole('button', { name: 'Map', exact: true }));
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  const planGlyph = await glyph(vehicle.getByRole('dialog').getByRole('button', { name: 'Plan', exact: true }));
  expect(planGlyph).not.toBeNull();
  expect(planGlyph, 'Oleg read the two as the same map and could not tell which one plans a mission').not.toEqual(mapGlyph);
});

test('Calibrate in the Fly menu opens the sensor calibrations', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  await vehicle.getByRole('dialog').getByRole('region', { name: 'Aircraft' }).getByRole('button', { name: 'Calibrate', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: /^Compass/ }).first(), 'compass, accelerometer and level are a tap away from the Fly screen').toBeVisible({ timeout: 30_000 });
  await expect(vehicle.getByRole('button', { name: /^Accelerometer/ }).first()).toBeVisible();
});

test('an ArduPilot with a range finder set up turns it off and on from the Fly menu, and armed it asks before turning it off', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  const menu = async () => {
    await app.getByRole('button', { name: 'Menu', exact: true }).click();
    return app.getByRole('dialog').getByRole('region', { name: 'Aircraft' });
  };
  await expect((await menu()).getByRole('button', { name: /^Range finder/ }), 'no range finder set up, no switch for it').toHaveCount(0);
  await app.keyboard.press('Escape');
  await expect
    .poll(
      () =>
        app.evaluate(async (module) => {
          const { Qgc } = (await import(module)) as { Qgc: { set: (path: string, value: unknown) => Promise<unknown>; get: (path: string) => Promise<{ value?: unknown }> } };
          await Qgc.set('vehicle.parameterManager.getParameter(-1,RNGFND1_TYPE)', 14);
          return (await Qgc.get('vehicle.parameterManager.getParameter(-1,RNGFND1_TYPE)')).value;
        }, '/src/bridge/Qgc.ts'),
      { timeout: 30_000 },
    )
    .toBe(14);
  const turn = (state: 'on' | 'off') => async () => (await menu()).getByRole('button', { name: `Turn range finder ${state}`, exact: true });
  const offered = async (state: 'on' | 'off') => {
    await expect(await turn(state)(), `the vehicle accepted it, so the menu now offers turning it ${state}`).toBeVisible({ timeout: 15_000 });
    await app.keyboard.press('Escape');
  };
  await (await turn('off')()).click();
  await offered('on');
  await (await turn('on')()).click();
  await offered('off');
  await expect((await menu()).getByRole('button', { name: /obstacle avoidance/ }), 'no proximity sensor set up, no avoidance switch').toHaveCount(0);
  await app.keyboard.press('Escape');
  await takeOff(app, '5 m');
  await expect(flightMode(app)).toHaveAccessibleName(/ Flying/, { timeout: 60_000 });
  await (await turn('off')()).click();
  const slide = app.getByRole('button', { name: 'Slide to turn range finder off', exact: true });
  await expect(slide, 'armed, a tap alone does not turn a sensor off').toBeVisible();
  await expect(app.getByText('The aircraft stops reading its height above the ground from the range finder.')).toBeVisible();
  await slideAcross(app, slide);
  await offered('on');
  await (await turn('on')()).click();
  await expect(app.getByRole('button', { name: /^Slide to turn range finder/ }), 'turning it back on asks nothing').toHaveCount(0);
  await offered('off');
});

test('on the ground Arm sits on the deck beside Take off, and Force arm stays in the menu in plain ink and still asks for a slide', async ({ vehicle }) => {
  const deck = vehicle.getByRole('group', { name: 'Flight actions' });
  await expect(deck.getByRole('button', { name: 'Arm', exact: true })).toBeVisible();
  await expect(deck.getByRole('button', { name: 'Take off' })).toBeVisible();
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  const aircraft = vehicle.getByRole('dialog').getByRole('region', { name: 'Aircraft' });
  const ink = (name: string) => aircraft.getByRole('button', { name, exact: true }).evaluate((tile) => getComputedStyle(tile).color);
  await expect(aircraft.getByRole('button', { name: 'Force arm', exact: true })).toBeVisible();
  expect(await ink('Force arm'), 'bypassing the arming checks does not shout louder than the other aircraft actions').toBe(await ink('All status'));
  await aircraft.getByRole('button', { name: 'Force arm', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: 'Slide to force arm' })).toBeVisible();
});

test('a take-off height being chosen survives the window narrowing to a phone and widening back', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'Take off' }).click();
  await vehicle.getByRole('button', { name: '50 m', exact: true }).click();
  const slide = vehicle.getByRole('button', { name: /^Slide to take off( anyway)? · 50(\.0)? m$/ });
  await expect(slide).toBeVisible();
  await vehicle.setViewportSize({ width: 412, height: 860 });
  await expect(slide, 'the panel and its height outlive the phone layout taking over').toBeVisible();
  await vehicle.setViewportSize({ width: 1280, height: 800 });
  await expect(slide).toBeVisible();
});

test('the keyboard follows a take-off: into the panel when it opens, and back onto the deck once the aircraft is up', async ({ app: vehicle }) => {
  test.setTimeout(120_000);
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await vehicle.getByRole('button', { name: 'Take off' }).focus();
  await vehicle.keyboard.press('Enter');
  const panel = vehicle.getByRole('group', { name: 'Take off' });
  await expect(panel, 'focus is not dropped onto the page when the button it was on gives way to the panel').toBeFocused();
  const slide = panel.getByRole('button', { name: /^Slide to take off anyway/ });
  await expect(slide, 'the simulated copter is never fully ready, so the label is settled before the hold starts').toBeVisible();
  await slide.focus();
  await vehicle.keyboard.down('Enter');
  await expect(slide).toBeHidden({ timeout: 30_000 });
  await vehicle.keyboard.up('Enter');
  await expect.poll(() => vehicle.evaluate(() => document.activeElement?.closest('[role="group"][aria-label="Flight actions"]') !== null), { message: 'the deck that replaced the panel takes the focus back' }).toBe(true);
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
  await expect(chooser.getByText('Altitude above home', { exact: true })).toBeVisible();
  await expect(chooser.getByText(/Alt \(|esimate|mainfold|BarometricPressure|^Vibe |spd|Next WP|Hobbs|^Imu |X track|Range finder dist|^Is stale$|^Manager compid$/), 'every reading reads as words').toHaveCount(0);
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

test('the payload sheet names itself, says a safe payload cannot drop and arms it in place', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Payload').click();
  const sheet = vehicle.getByRole('dialog', { name: 'Payload' });
  await expect(sheet.getByText('Payload', { exact: true })).toBeVisible();
  await expect(sheet.getByText('The payload is safe. Arm it to drop.')).toBeVisible();
  await expect(sheet.getByRole('button', { name: 'Slide to drop' }), 'a drop the rail would refuse is not offered').toBeDisabled();
  await sheet.getByRole('button', { name: 'Arm payload' }).click();
  await expect(sheet.getByRole('button', { name: 'Make safe' })).toBeVisible();
  await expect(sheet.getByRole('button', { name: 'Slide to drop' })).toBeEnabled();
  await vehicle.keyboard.press('Escape');
  await expect(vehicle.getByRole('group', { name: 'Payload', exact: true }).getByRole('button', { name: 'Payload armed, click to make safe' }), 'it is the same lock as the strip').toBeVisible();
});

test('in flight the payload strip is the only drop: the deck offers none, and the strip drops once armed', async ({ vehicle }) => {
  test.setTimeout(120_000);
  await expect(vehicle.getByRole('button', { name: 'Take off' })).toBeVisible();
  await takeOff(vehicle, '5 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await expect(vehicle.getByRole('group', { name: 'Flight actions' }).getByRole('button', { name: 'Return home', exact: true })).toBeVisible();
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

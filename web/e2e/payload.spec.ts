import type { Locator, Page } from '@playwright/test';
import { armPayload, closeSettings, connectMock, dropFromStrip, expect, flightMode, itemCount, openMenu, openPlanAtVehicle, showMapFull, slideAcross, takeOff, tapPlanMission, test, vehicleStatus } from './fixtures';

test.describe.configure({ timeout: 180_000 });

const STAYS_OPEN = /The gripper stays open after the first drop, so the second drop releases nothing/;

test('a mission whose second drop would not fall says why and turns on auto-close in one tap', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  const drop = vehicle.getByRole('button', { name: 'Drop', exact: true });
  await drop.click();
  await drop.click();
  await vehicle.getByRole('button', { name: 'Upload', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: 'Upload', exact: true })).toBeHidden({ timeout: 30_000 });
  await vehicle.getByRole('button', { name: 'Fly', exact: true }).last().click();

  const status = await vehicleStatus(vehicle);
  await expect(status.getByText(STAYS_OPEN)).toBeVisible({ timeout: 30_000 });
  await vehicle.keyboard.press('Escape');
  await expect(vehicle.getByRole('button', { name: 'Slide to start mission' })).toBeHidden();

  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Payload').click();
  const panel = vehicle.getByRole('dialog').filter({ hasText: STAYS_OPEN });
  await expect(panel.getByRole('button', { name: 'Turn on auto-close' })).toBeVisible();
  await vehicle.keyboard.press('Escape');
  await expect(panel).toBeHidden();

  const sheet = (await vehicleStatus(vehicle)).filter({ hasText: STAYS_OPEN });
  await sheet.getByRole('button', { name: 'Turn on auto-close' }).click();
  await expect(vehicle.getByText(STAYS_OPEN)).toBeHidden({ timeout: 30_000 });
  await vehicle.keyboard.press('Escape');
  await expect(vehicle.getByText('Start mission', { exact: true })).toBeVisible({ timeout: 30_000 });
});

test('a drop after the last waypoint falls and ends the mission', async ({ app: vehicle }) => {
  test.setTimeout(300_000);
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await vehicle.getByRole('button', { name: 'Drop', exact: true }).click();
  await vehicle.getByRole('button', { name: 'Upload', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: 'Upload', exact: true })).toBeHidden({ timeout: 30_000 });
  await vehicle.getByRole('button', { name: 'Fly', exact: true }).last().click();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to start mission' }));
  const progress = vehicle.getByText(/^To waypoint \d+ of \d+/);
  await expect(progress).toBeVisible({ timeout: 30_000 });
  await expect(progress).toBeHidden({ timeout: 240_000 });
  await armPayload(vehicle);
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Payload').click();
  await slideAcross(vehicle, vehicle.getByRole('dialog').getByRole('button', { name: 'Slide to drop' }));
  await expect(vehicle.getByText('The gripper is still open from the last drop, so nothing would fall. Grab first, or turn on auto-close.')).toBeVisible();
});

test('a second drop while the gripper is still open is refused rather than counted', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await armPayload(vehicle);
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Payload').click();
  const sheet = vehicle.getByRole('dialog');
  await slideAcross(vehicle, sheet.getByRole('button', { name: 'Slide to drop' }));
  await expect(vehicle.getByText(/^Dropped \d+ s ago$/)).toBeVisible({ timeout: 30_000 });
  await slideAcross(vehicle, sheet.getByRole('button', { name: 'Slide to drop' }));
  await expect(vehicle.getByText('The gripper is still open from the last drop, so nothing would fall. Grab first, or turn on auto-close.')).toBeVisible();
  await sheet.getByRole('button', { name: 'Grab', exact: true }).click();
  await expect(vehicle.getByText(/^Grabbed \d+ s ago$/)).toBeVisible();
  await slideAcross(vehicle, sheet.getByRole('button', { name: 'Slide to drop' }));
  await expect(vehicle.getByText(/^Dropped \d+ s ago$/)).toBeVisible({ timeout: 30_000 });
});

test('a waypoint drops from its own servo after a wait, a second waypoint is told the bay is used, and the drop goes with its wait', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await expect.poll(() => itemCount(vehicle)).toBeGreaterThanOrEqual(5);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await vehicle.getByRole('button', { name: 'Waypoint 3' }).first().click();
  const before = await itemCount(vehicle);
  await vehicle.getByRole('button', { name: 'Add action', exact: true }).click();
  await vehicle.getByRole('menuitem', { name: 'Drop payload' }).click();
  await expect.poll(() => itemCount(vehicle), { timeout: 30_000 }).toBe(before + 1);
  const editor = vehicle.getByRole('dialog').filter({ hasText: 'Wait before dropping' });
  await editor.getByRole('button', { name: 'Servo', exact: true }).click();
  const typed = async (label: string, value: string) => {
    await editor.getByRole('button', { name: new RegExp(`^${label}: `) }).click();
    await vehicle.getByRole('textbox', { name: new RegExp(`^${label}`) }).fill(value);
    await vehicle.getByRole('button', { name: 'Set', exact: true }).click();
  };
  await typed('Servo output', '4');
  await typed('Release PWM', '2000');
  await typed('Wait before dropping', '10');
  await editor.getByRole('button', { name: 'Done' }).click();
  await expect(vehicle.getByText('Servo 4 to 2000 · after 10 s'), 'the chip reads the mission item: servo 4, opened at 2000 after the drone has waited 10 s').toBeVisible({ timeout: 30_000 });
  await expect.poll(() => itemCount(vehicle), { message: 'the drop and the wait before it', timeout: 30_000 }).toBe(before + 2);

  await vehicle.getByRole('button', { name: 'Waypoint 2' }).first().click();
  await vehicle.getByRole('button', { name: 'Add action', exact: true }).click();
  await vehicle.getByRole('menuitem', { name: 'Drop payload' }).click();
  await expect(vehicle.getByText(/Every bay already drops in this mission/), 'the one bay is taken, and the pilot is told instead of nothing happening').toBeVisible({ timeout: 30_000 });
  await expect(vehicle.getByRole('dialog').filter({ hasText: 'Wait before dropping' }), 'no editor opens for a drop that was not added').toHaveCount(0);
  expect(await itemCount(vehicle)).toBe(before + 2);

  await vehicle.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await expect(vehicle.getByRole('button', { name: 'Remove Drop payload' })).toBeVisible();
  await vehicle.getByRole('button', { name: 'Remove Drop payload' }).click();
  await expect.poll(() => itemCount(vehicle), { timeout: 30_000 }).toBe(before);
  await expect(vehicle.getByRole('button', { name: 'Add action', exact: true })).toBeVisible();
});

test('deleting a waypoint takes its wait and its drop with it, so the waypoint before does not drop instead', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await expect.poll(() => itemCount(vehicle)).toBeGreaterThanOrEqual(5);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await vehicle.getByRole('button', { name: 'Waypoint 3' }).first().click();
  const before = await itemCount(vehicle);
  await vehicle.getByRole('button', { name: 'Add action', exact: true }).click();
  await vehicle.getByRole('menuitem', { name: 'Drop payload' }).click();
  const editor = vehicle.getByRole('dialog').filter({ hasText: 'Wait before dropping' });
  await editor.getByRole('button', { name: /^Wait before dropping: / }).click();
  await vehicle.getByRole('textbox', { name: /^Wait before dropping/ }).fill('10');
  await vehicle.getByRole('button', { name: 'Set', exact: true }).click();
  await editor.getByRole('button', { name: 'Done' }).click();
  await expect.poll(() => itemCount(vehicle), { timeout: 30_000 }).toBe(before + 2);
  await vehicle.getByRole('button', { name: 'Delete waypoint 3' }).click();
  await expect.poll(() => itemCount(vehicle), { message: 'the waypoint, its wait and its drop', timeout: 30_000 }).toBe(before - 1);
  await vehicle.getByRole('button', { name: 'Waypoint 2' }).first().click();
  await expect(vehicle.getByRole('button', { name: 'Remove Drop payload' }), 'waypoint 2 did not inherit the drop').toHaveCount(0);
});

const planNames = (page: Page): Promise<readonly string[]> =>
  page.evaluate(async (module) => {
    const view = (await ((await import(module)) as { Qgc: { get: (path: string) => Promise<unknown> } }).Qgc.get('view.missionItems(geometry)')) as { items?: { name?: string }[] } | null;
    return (view?.items ?? []).map((it) => it.name ?? '');
  }, '/src/bridge/Qgc.ts');

const afterWaypoint = (names: readonly string[], nth: number): readonly string[] => {
  const starts = names.flatMap((name, at) => (name === 'Waypoint' ? [at] : []));
  const from = starts[nth - 1] ?? names.length;
  return names.slice(from + 1, starts[nth] ?? names.length);
};

test('the pilot sets the order of a waypoint\'s actions: aim, photo, hold, drop at one, and servo before camera at the next', async ({ app: vehicle }) => {
  test.setTimeout(240_000);
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await expect.poll(() => itemCount(vehicle)).toBeGreaterThanOrEqual(5);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await vehicle.evaluate(async (module) => ((await import(module)) as { Qgc: { call: (path: string, ...args: unknown[]) => Promise<unknown> } }).Qgc.call('advancedUi.set', true), '/src/bridge/Qgc.ts');
  const typed = async (editor: Locator, label: string, value: string) => {
    await editor.getByRole('button', { name: new RegExp(`^${label}: `) }).click();
    await vehicle.getByRole('textbox', { name: new RegExp(`^${label}`) }).fill(value);
    await vehicle.getByRole('button', { name: 'Set', exact: true }).click();
  };
  const add = async (action: string) => {
    await vehicle.getByRole('button', { name: 'Add action', exact: true }).click();
    await vehicle.getByRole('menuitem', { name: action }).click();
    return vehicle.getByRole('dialog');
  };
  const order = vehicle.getByRole('list', { name: 'Actions in order' }).getByRole('listitem');

  await vehicle.getByRole('button', { name: 'Waypoint 1' }).first().click();
  const hover = await add('Hover');
  await typed(hover, 'Hover', '10');
  await hover.getByRole('button', { name: 'Done' }).click();
  const tilt = await add('Tilt camera');
  await typed(tilt, 'Yaw', '60');
  await tilt.getByRole('button', { name: 'Done' }).click();
  await (await add('Photo')).getByRole('button', { name: 'Done' }).click();
  const drop = await add('Drop payload');
  await drop.getByRole('button', { name: 'Servo', exact: true }).click();
  await typed(drop, 'Servo output', '4');
  await typed(drop, 'Release PWM', '1500');
  await drop.getByRole('button', { name: 'Done' }).click();
  await expect(order).toHaveText([/^Hover/, /^Gimbal/, /^Take photo/, /^Drop payload/]);
  await vehicle.getByRole('button', { name: 'Move Hover later' }).click();
  await expect(order).toHaveText([/^Gimbal/, /^Wait/, /^Take photo/, /^Drop payload/], { timeout: 15_000 });
  await vehicle.getByRole('button', { name: 'Move Wait later' }).click();
  await expect(order, 'camera yaw 60, photo, hold 10 s, then servo 4 to 1500').toHaveText([/^Gimbal0° \/ 60°/, /^Take photo/, /^Wait10 s/, /^Drop payloadServo 4 to 1500/], { timeout: 15_000 });
  await vehicle.screenshot({ path: process.env.ORDER_SHOT ?? 'test-results/order-first-waypoint.png' });
  await expect.poll(async () => afterWaypoint(await planNames(vehicle), 1).map((name) => (/delay/i.test(name) ? 'delay' : /servo|drop/i.test(name) ? 'servo' : name)), { message: 'the mission holds the delay and then the servo after waypoint 1, with the camera folded ahead of them', timeout: 15_000 }).toEqual(['delay', 'servo']);

  await expect(vehicle.getByRole('button', { name: /^Delay/i }), 'the hold now has a row of its own in the list').toBeVisible({ timeout: 15_000 });
  await vehicle.getByRole('button', { name: /^2 Waypoint\b/ }).click();
  const aim = await add('Tilt camera');
  await typed(aim, 'Pitch', '-20');
  await typed(aim, 'Yaw', '90');
  await aim.getByRole('button', { name: 'Done' }).click();
  await vehicle.getByRole('button', { name: 'Show advanced settings' }).click();
  await vehicle.getByRole('button', { name: 'Change command' }).click();
  const picker = vehicle.getByRole('dialog').filter({ hasText: 'Select mission command' });
  await picker.getByText('Advanced', { exact: true }).click();
  await picker.getByText('Set servo', { exact: true }).click();
  await expect(vehicle.getByRole('button', { name: /^Set servo\b/ })).toBeVisible({ timeout: 15_000 });
  await vehicle.getByRole('button', { name: /^2 Waypoint\b/ }).click();
  await expect(order).toHaveText([/^Gimbal/, /^Drop payload/], { timeout: 15_000 });
  await order.nth(1).getByRole('button').first().click();
  const servo = vehicle.getByRole('dialog').filter({ hasText: 'Servo output' });
  await typed(servo, 'Servo output', '4');
  await typed(servo, 'Release PWM', '2000');
  await servo.getByRole('button', { name: 'Done' }).click();
  await vehicle.getByRole('button', { name: 'Move Drop payload earlier' }).click();
  await expect(order, 'servo to 2000, then camera yaw 90, 20 down').toHaveText([/^Drop payloadServo 4 to 2000/, /^Gimbal-20° \/ 90°/], { timeout: 15_000 });
  await expect.poll(async () => afterWaypoint(await planNames(vehicle), 2).map((name) => (/servo|drop/i.test(name) ? 'servo' : /mount|gimbal/i.test(name) ? 'camera' : name)), { message: 'after waypoint 2 the mission holds the servo and then the camera', timeout: 15_000 }).toEqual(['servo', 'camera']);
  await vehicle.screenshot({ path: process.env.ORDER_SHOT_2 ?? 'test-results/order-second-waypoint.png' });

  await vehicle.getByRole('button', { name: /^1 Waypoint\b/ }).click();
  await expect(order, 'waypoint 1 kept its own order').toHaveText([/^Gimbal/, /^Take photo/, /^Wait/, /^Drop payload/], { timeout: 15_000 });
});

test('a pilot flying by hand drops from the payload strip, and a drop that cannot fall says why', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await takeOff(vehicle, '10 m');
  await expect(vehicle.getByRole('button', { name: 'Slide to land' })).toBeVisible({ timeout: 60_000 });
  await vehicle.getByRole('button', { name: 'Change flight mode', exact: true }).click();
  await vehicle.getByRole('menu').getByRole('menuitem', { name: 'Loiter' }).click();
  const asked = vehicle.getByRole('dialog').filter({ hasText: 'Set the vehicle flight mode to Loiter. Flown with the RC transmitter, which has no signal.' });
  await asked.getByRole('button', { name: 'Confirm' }).click();
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Loiter/);
  await dropFromStrip(vehicle);
  await expect(vehicle.getByText('Payload dropped')).toBeVisible({ timeout: 30_000 });
  await dropFromStrip(vehicle);
  await expect(vehicle.getByText('The gripper is still open from the last drop, so nothing would fall. Grab first, or turn on auto-close.').first()).toBeVisible({ timeout: 30_000 });
  await expect(vehicle.getByRole('status').filter({ hasText: 'Payload dropped' }), 'the refusal is the newest payload result, so the earlier drop leaves the screen').toBeHidden({ timeout: 1_000 });
});

const BAY_OUTPUTS = [9, 10, 11, 12] as const;
const GRIP_RELEASE_PWM = 1100;

const fourServoBays = async (page: Page) => {
  await openMenu(page, 'Settings');
  await page.getByPlaceholder('Search settings').fill('bay');
  const bays = page.getByRole('textbox', { name: /^(How many payload bays|Payload bays)/ });
  await bays.fill('4');
  await bays.press('Enter');
  await page.getByRole('radio', { name: 'A servo per bay' }).click();
  await expect(page.getByText('Bay 4 output', { exact: true })).toBeVisible();
  await closeSettings(page);
};

const bayPwms = (page: Page): Promise<readonly (number | null)[]> =>
  page.evaluate(
    async ([module, outputs]) => {
      const { Qgc } = (await import(module)) as { Qgc: { get: (path: string) => Promise<{ servos?: { index: number; pwm: number | null }[] } | null> } };
      const servos = (await Qgc.get('view.apmServos'))?.servos ?? [];
      return outputs.map((output) => servos.find((it) => it.index === output)?.pwm ?? null);
    },
    ['/src/bridge/Qgc.ts', BAY_OUTPUTS] as const,
  );

const expectBaysReleased = (page: Page) => expect.poll(() => bayPwms(page), { timeout: 30_000 }).toEqual(BAY_OUTPUTS.map(() => GRIP_RELEASE_PWM));

test('four bays on their own servos drop one after another by hand, each from its own output', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await fourServoBays(vehicle);
  await takeOff(vehicle, '10 m');
  await expect(vehicle.getByRole('button', { name: 'Slide to land' })).toBeVisible({ timeout: 60_000 });
  await vehicle.getByRole('button', { name: 'Change flight mode', exact: true }).click();
  await vehicle.getByRole('menu').getByRole('menuitem', { name: 'Loiter' }).click();
  await vehicle.getByRole('dialog').getByRole('button', { name: 'Confirm' }).click();
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Loiter/);
  await armPayload(vehicle);
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Payload').click();
  const sheet = vehicle.getByRole('dialog');
  await expect(sheet.getByText('Bay 2 · output 10', { exact: true })).toBeVisible();
  await [1, 2, 3, 4].reduce(async (previous, bay) => {
    await previous;
    await slideAcross(vehicle, sheet.getByRole('button', { name: `Slide to drop bay ${bay}` }));
    await expect(sheet.getByText(new RegExp(`^Bay ${bay} dropped \\d+ s ago$`))).toBeVisible({ timeout: 30_000 });
  }, Promise.resolve());
  await expect(sheet.getByText(/still open/)).toBeHidden();
  await vehicle.keyboard.press('Escape');
  await expectBaysReleased(vehicle);
});

test('dropping bays quickly from the payload strip shows each bay as it drops, never one left queued by an earlier drop', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await fourServoBays(vehicle);
  await takeOff(vehicle, '10 m');
  await expect(vehicle.getByRole('button', { name: 'Slide to land' })).toBeVisible({ timeout: 60_000 });
  const notice = vehicle.getByRole('status').filter({ hasText: / dropped$/ });
  await [1, 2, 3].reduce(async (previous, bay) => {
    await previous;
    await dropFromStrip(vehicle, bay);
    await expect(notice, `the notice on screen after dropping bay ${bay} names bay ${bay}`).toHaveText([`Bay ${bay} dropped`], { timeout: 1_500 });
  }, Promise.resolve());
});

test('a planned four-drop mission on servo bays uploads, flies and drops each bay once', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await fourServoBays(vehicle);
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await vehicle.getByRole('button', { name: 'Waypoint 2' }).first().click();
  const drop = vehicle.getByRole('button', { name: 'Drop', exact: true });
  await [1, 2, 3, 4].reduce(async (previous) => {
    await previous;
    const before = await itemCount(vehicle);
    await drop.click();
    await expect.poll(() => itemCount(vehicle), { timeout: 30_000 }).toBe(before + 1);
  }, Promise.resolve());
  await expect(vehicle.getByText('Drop bay 4').first()).toBeVisible();
  await vehicle.getByRole('button', { name: 'Upload', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: 'Upload', exact: true })).toBeHidden({ timeout: 30_000 });
  await vehicle.getByRole('button', { name: 'Fly', exact: true }).last().click();
  await showMapFull(vehicle);
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to start mission' }));
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Payload').click();
  const sheet = vehicle.getByRole('dialog');
  await [1, 2, 3, 4].reduce(async (previous, bay) => {
    await previous;
    await expect(sheet.getByRole('button', { name: `Bay ${bay} empty, slide to drop` })).toBeVisible({ timeout: 120_000 });
  }, Promise.resolve());
  await vehicle.keyboard.press('Escape');
  await expectBaysReleased(vehicle);
});

import type { Locator, Page } from '@playwright/test';
import { actionGrip, actionOrder, armPayload, centreOf, closeSettings, connectMock, dropFromStrip, expect, flightMode, itemCount, keyboardMoveAction, mouseDragAction, openMenu, openPlanAtVehicle, savedMission, showMapFull, slideAcross, takeOff, tapPlanMission, test, touchDragAction, vehicleStatus } from './fixtures';

test.describe.configure({ timeout: 180_000 });
test.use({ video: process.env.E2E_VIDEO === undefined ? 'off' : { mode: 'on', size: { width: 1280, height: 800 } } });

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
  await expect(vehicle.getByText('Servo 4 to 2000', { exact: true }), 'the chip reads the mission item: servo 4, opened at 2000').toBeVisible({ timeout: 30_000 });
  await expect(actionOrder(vehicle), 'the wait before it is its own row, so the drop does not repeat it').toHaveText([/^Hover2 s/, /^Wait10 s/, /^Drop payloadServo 4 to 2000/], { timeout: 30_000 });
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

const NAV_WAYPOINT = 16;
const MOUNT = 205;
const PHOTO = 2000;
const DELAY = 93;
const SERVO = 183;
const PROOF = process.env.DRAG_PROOF;
const WRONG_ROW = /^Delete (delay|wait|take photo|gimbal|drop|set servo)/i;

const afterWaypoint = (commands: readonly number[], nth: number): readonly number[] => {
  const starts = commands.flatMap((command, at) => (command === NAV_WAYPOINT && at > 0 ? [at] : []));
  const from = starts[nth - 1] ?? commands.length;
  return commands.slice(from + 1, starts[nth] ?? commands.length);
};

const typedInto = async (page: Page, editor: Locator, label: string, value: string) => {
  await editor.getByRole('button', { name: new RegExp(`^${label}: `) }).click();
  await page.getByRole('textbox', { name: new RegExp(`^${label}`) }).fill(value);
  await page.getByRole('button', { name: 'Set', exact: true }).click();
};

const addAction = async (page: Page, action: string): Promise<Locator> => {
  await page.getByRole('button', { name: 'Add action', exact: true }).click();
  await page.getByRole('menuitem', { name: action }).click();
  return page.getByRole('dialog');
};

const aimedWaypoint = async (page: Page, waypoint: number, hover: string, yaw: string) => {
  await page.getByRole('button', { name: `Waypoint ${waypoint}` }).first().click();
  const hold = await addAction(page, 'Hover');
  await typedInto(page, hold, 'Hover', hover);
  await hold.getByRole('button', { name: 'Done' }).click();
  const tilt = await addAction(page, 'Tilt camera');
  await typedInto(page, tilt, 'Yaw', yaw);
  await tilt.getByRole('button', { name: 'Done' }).click();
  await (await addAction(page, 'Photo')).getByRole('button', { name: 'Done' }).click();
};

const planned = async (page: Page) => {
  await connectMock(page, [], 'APM ArduCopter Vehicle');
  await openPlanAtVehicle(page);
  await tapPlanMission(page);
  await expect.poll(() => itemCount(page)).toBeGreaterThanOrEqual(5);
  await page.getByRole('button', { name: 'Done' }).click();
};

test('the pilot drags the actions into his order: aim, photo, hold 10 s, drop at one waypoint, and the servo before the camera at the next', async ({ app: vehicle }) => {
  test.setTimeout(240_000);
  await planned(vehicle);
  const order = actionOrder(vehicle);
  await aimedWaypoint(vehicle, 1, '10', '60');
  const drop = await addAction(vehicle, 'Drop payload');
  await drop.getByRole('button', { name: 'Servo', exact: true }).click();
  await typedInto(vehicle, drop, 'Servo output', '4');
  await typedInto(vehicle, drop, 'Release PWM', '1500');
  await drop.getByRole('button', { name: 'Done' }).click();
  await expect(order).toHaveText([/^Hover10 s/, /^Gimbal0° \/ 60°/, /^Take photo/, /^Drop payloadServo 4 to 1500/]);
  await expect.poll(async () => afterWaypoint(await savedMission(vehicle), 1), { message: 'as added, the hold is the waypoint\'s own and the camera and servo follow it' }).toEqual([MOUNT, PHOTO, SERVO]);

  await mouseDragAction(vehicle, 'Hover', order.nth(2), {
    whileHeld: async () => {
      await expect(vehicle.getByText('Wait', { exact: true }).first(), 'off first place the hold already reads as a wait while it is held').toBeVisible();
      if (PROOF) await vehicle.screenshot({ path: `${PROOF}/drag-mid.png` });
    },
  });
  await expect(order, 'camera yaw 60, photo, hold on this position 10 s, then servo 4 to 1500').toHaveText([/^Gimbal0° \/ 60°/, /^Take photo/, /^Wait10 s/, /^Drop payloadServo 4 to 1500/], { timeout: 15_000 });
  await expect.poll(async () => afterWaypoint(await savedMission(vehicle), 1), { message: 'one drag, one move: the delay and then the servo follow the camera' }).toEqual([MOUNT, PHOTO, DELAY, SERVO]);
  await expect(vehicle.getByText('Drop payload', { exact: true }), 'the lifted copy has settled into the list').toHaveCount(1);
  if (PROOF) await vehicle.screenshot({ path: `${PROOF}/drag-after.png` });

  await expect(vehicle.getByRole('button', { name: /^Delay/i }).first(), 'the plan list has the wait as its own row').toBeVisible({ timeout: 15_000 });
  await vehicle.getByRole('button', { name: /^2 Waypoint\b/ }).click();
  const aim = await addAction(vehicle, 'Tilt camera');
  await typedInto(vehicle, aim, 'Pitch', '-20');
  await typedInto(vehicle, aim, 'Yaw', '90');
  await aim.getByRole('button', { name: 'Done' }).click();
  await vehicle.getByRole('button', { name: 'Show advanced settings' }).click();
  await vehicle.getByRole('button', { name: 'Change command' }).click();
  const picker = vehicle.getByRole('dialog').filter({ hasText: 'Select mission command' });
  await picker.getByText('Advanced', { exact: true }).click();
  await picker.getByText('Set servo', { exact: true }).click();
  await expect(vehicle.getByRole('button', { name: /^Set servo\b/ })).toBeVisible({ timeout: 15_000 });
  await vehicle.getByRole('button', { name: /^2 Waypoint\b/ }).click();
  await expect(order).toHaveText([/^Gimbal/, /^Drop payload/], { timeout: 15_000 });
  await order.nth(1).getByRole('button', { name: /^Drop payload/ }).click();
  const servo = vehicle.getByRole('dialog').filter({ hasText: 'Servo output' });
  await typedInto(vehicle, servo, 'Servo output', '4');
  await typedInto(vehicle, servo, 'Release PWM', '2000');
  await servo.getByRole('button', { name: 'Done' }).click();
  await mouseDragAction(vehicle, 'Drop payload', order.nth(0));
  await expect(order, 'servo to 2000, then camera yaw 90, 20 down').toHaveText([/^Drop payloadServo 4 to 2000/, /^Gimbal-20° \/ 90°/], { timeout: 15_000 });
  await expect.poll(async () => afterWaypoint(await savedMission(vehicle), 2)).toEqual([SERVO, MOUNT]);

  await vehicle.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(order, 'one undo puts back the order from before the drag').toHaveText([/^Gimbal/, /^Drop payload/], { timeout: 15_000 });
  await expect.poll(async () => afterWaypoint(await savedMission(vehicle), 2)).toEqual([MOUNT, SERVO]);
  await vehicle.getByRole('button', { name: /^1 Waypoint\b/ }).click();
  await expect(order, 'waypoint 1 kept its own order').toHaveText([/^Gimbal/, /^Take photo/, /^Wait/, /^Drop payload/], { timeout: 15_000 });
});

test('an order the aircraft cannot fly opens no slot and sends nothing, and dragging out of the list or Escape puts the row back', async ({ app: vehicle }) => {
  await planned(vehicle);
  const order = actionOrder(vehicle);
  await aimedWaypoint(vehicle, 1, '0.5', '30');
  await expect(order).toHaveText([/^Hover0\.5 s/, /^Gimbal/, /^Take photo/]);
  const before = afterWaypoint(await savedMission(vehicle), 1);
  await mouseDragAction(vehicle, 'Take photo', order.nth(0), {
    whileHeld: async () => {
      await expect(vehicle.getByText('Hover stays first. To wait later in the list, make it 1 s or longer.', { exact: true }), 'held over the hold, the photo says why it cannot go there').toBeVisible();
      await expect(order.nth(0), 'and no slot opens: the hold stays first').toHaveText(/^Hover/);
      await expect(order.nth(0), 'the hold is the row the reason is about, and it is marked so').toHaveAccessibleDescription(/^Hover stays first/);
      const hold = await centreOf(order.nth(0));
      expect(await vehicle.evaluate(([x, y]) => document.elementFromPoint(x, y)?.closest('li')?.textContent ?? '', [hold.x, hold.y] as const), 'and it stays in view, above the photo held over it').toMatch(/^Hover0\.5 s/);
      await expect.poll(() => order.evaluateAll((rows) => rows.map((row) => new DOMMatrix(getComputedStyle(row).transform).m42)), { message: 'and no row slides aside to make room' }).toEqual([0, 0, 0]);
      if (PROOF) await vehicle.screenshot({ path: `${PROOF}/drag-refused.png` });
    },
  });
  await expect(order).toHaveText([/^Hover0\.5 s/, /^Gimbal/, /^Take photo/]);
  expect(afterWaypoint(await savedMission(vehicle), 1), 'nothing was sent').toEqual(before);

  const map = (await vehicle.getByRole('region', { name: 'Map' }).boundingBox())!;
  await mouseDragAction(vehicle, 'Take photo', order.nth(1), {
    whileHeld: async () => {
      await vehicle.mouse.move(map.x + map.width / 2, map.y + map.height / 2, { steps: 8 });
      await expect(vehicle.getByText('Let go to put it back.', { exact: true })).toBeVisible();
    },
  });
  await expect(order, 'let go over the map, the photo is back in its place').toHaveText([/^Hover/, /^Gimbal/, /^Take photo/]);
  await mouseDragAction(vehicle, 'Take photo', order.nth(1), { whileHeld: () => vehicle.keyboard.press('Escape') });
  await expect(order, 'Escape put it back').toHaveText([/^Hover/, /^Gimbal/, /^Take photo/]);
  expect(afterWaypoint(await savedMission(vehicle), 1)).toEqual(before);
});

test('a held finger lifts the row, the rows slide aside as it passes, and letting go moves it in one step', async ({ app: vehicle }) => {
  await planned(vehicle);
  const order = actionOrder(vehicle);
  await aimedWaypoint(vehicle, 1, '10', '45');
  await touchDragAction(vehicle, 'Take photo', order.nth(2), order.nth(0));
  await expect(order, 'the photo first, and the hold it pushed down is now a wait').toHaveText([/^Take photo/, /^Wait10 s/, /^Gimbal/], { timeout: 15_000 });
  await expect.poll(async () => afterWaypoint(await savedMission(vehicle), 1)).toEqual([PHOTO, DELAY, MOUNT]);
  await expect(vehicle.getByRole('dialog'), 'the long press opened no editor when the finger lifted').toHaveCount(0);
});

test('the keyboard lifts an action from its grip, moves it with the arrows, drops it with Space and cancels with Escape, and says so', async ({ app: vehicle }) => {
  await planned(vehicle);
  const order = actionOrder(vehicle);
  await aimedWaypoint(vehicle, 1, '10', '45');
  await keyboardMoveAction(vehicle, 'Take photo', ['ArrowUp'], 'Escape');
  await expect(vehicle.locator('[id^="DndLiveRegion"]')).toHaveText(/cancelled/);
  await expect(order).toHaveText([/^Hover/, /^Gimbal/, /^Take photo/]);
  await keyboardMoveAction(vehicle, 'Hover', ['ArrowDown', 'ArrowDown']);
  await expect(vehicle.locator('[id^="DndLiveRegion"]')).toHaveText(/dropped as action 3 of 3/);
  await expect(order).toHaveText([/^Gimbal/, /^Take photo/, /^Wait10 s/], { timeout: 15_000 });
  await expect.poll(async () => afterWaypoint(await savedMission(vehicle), 1)).toEqual([MOUNT, PHOTO, DELAY]);
});

test('a wait is retimed from its own row, and a tap on the plan right after a move opens the waypoint the pilot saw', async ({ app: vehicle }) => {
  await planned(vehicle);
  const order = actionOrder(vehicle);
  await aimedWaypoint(vehicle, 1, '10', '45');
  await mouseDragAction(vehicle, 'Hover', order.nth(2));
  await vehicle.getByRole('button', { name: 'Waypoint 2' }).first().click();
  await expect(vehicle.getByRole('button', { name: 'Delete waypoint 2' }), 'one tap, straight after the move, opens waypoint 2').toBeVisible({ timeout: 15_000 });
  await expect(vehicle.getByRole('button', { name: WRONG_ROW }), 'and never the row that used to be waypoint 2').toHaveCount(0);
  await vehicle.getByRole('button', { name: /^1 Waypoint\b/ }).click();
  await expect(order).toHaveText([/^Gimbal/, /^Take photo/, /^Wait10 s/], { timeout: 15_000 });
  await order.nth(2).getByRole('button', { name: /^Wait/ }).click();
  const wait = vehicle.getByRole('dialog').filter({ hasText: 'The aircraft holds over the waypoint' });
  await typedInto(vehicle, wait, 'Wait', '25');
  await wait.getByRole('button', { name: 'Done' }).click();
  await expect(order).toHaveText([/^Gimbal/, /^Take photo/, /^Wait25 s/], { timeout: 15_000 });
  await expect.poll(async () => afterWaypoint(await savedMission(vehicle), 1)).toEqual([MOUNT, PHOTO, DELAY]);
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

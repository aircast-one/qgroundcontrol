import type { Page } from '@playwright/test';
import { connectMock, expect, itemCount, openApp, openMenu, openPlanAtVehicle, tapPlanMission, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

const planMenu = async (page: Page, item: string) => {
  await page.getByRole('button', { name: 'Plan menu' }).click();
  await page.getByRole('menuitem', { name: item }).click();
};

test('tapping the plan map builds a mission with home, takeoff and waypoints', async ({ app }) => {
  await openMenu(app, 'Plan');
  await expect(app.getByText('Click the map to add a waypoint')).toBeVisible();
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  const drawn = await itemCount(app);
  await app.getByRole('button', { name: 'Undo' }).click();
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBeLessThan(drawn);
});

test('a return goes on the end of the route, and a later tap still lands before it', async ({ app }) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.getByRole('button', { name: 'Waypoint 3' }).click();
  await app.getByRole('button', { name: 'Return', exact: true }).click();
  const lastRow = app.getByRole('button', { name: /^(H|T|R|\d+) (Launch|Takeoff|Waypoint|Return to launch)\b/ }).last();
  await expect(lastRow, 'with a waypoint selected, the return used to go in after it and strand the rest of the route').toHaveAccessibleName(/Return to launch/);
  await app.getByRole('button', { name: 'Done' }).click();
  const drawn = await itemCount(app);
  const box = await app.getByRole('region', { name: 'Map' }).boundingBox();
  if (box === null) throw new Error('plan map is not on screen');
  await expect(async () => {
    await app.mouse.click(box.x + box.width * 0.8, box.y + box.height * 0.25);
    await expect.poll(() => itemCount(app), { timeout: 10_000 }).toBeGreaterThan(drawn);
  }).toPass({ timeout: 30_000 });
  await expect(lastRow).toHaveAccessibleName(/Return to launch/);
  await expect(app.getByText(/after the route ends/)).toHaveCount(0);
});

test('with the new return still selected, a tap on the map lands before it, where it is flown', async ({ app }) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Return', exact: true }).click();
  const lastRow = app.getByRole('button', { name: /^(H|T|R|\d+) (Launch|Takeoff|Waypoint|Return to launch)\b/ }).last();
  await expect(lastRow).toHaveAccessibleName(/Return to launch/);
  await expect(app.getByText(/the map to add before Return to launch/), 'the hint names the item, not its mission sequence').toBeVisible();
  const drawn = await itemCount(app);
  const box = await app.getByRole('region', { name: 'Map' }).boundingBox();
  if (box === null) throw new Error('plan map is not on screen');
  await expect(async () => {
    await app.mouse.click(box.x + box.width * 0.8, box.y + box.height * 0.25);
    await expect.poll(() => itemCount(app), { timeout: 10_000 }).toBeGreaterThan(drawn);
  }).toPass({ timeout: 30_000 });
  await expect(lastRow, 'the waypoint used to go in after the return, listed after the route ends and never flown').toHaveAccessibleName(/Return to launch/);
  await expect(app.getByText(/after the route ends/)).toHaveCount(0);
});

const coreCall = (page: Page, path: string, ...args: unknown[]) =>
  page.evaluate(async ([module, called, given]) => ((await import(module)) as { Qgc: { call: (path: string, ...args: unknown[]) => Promise<unknown> } }).Qgc.call(called, ...given), ['/src/bridge/Qgc.ts', path, args] as const);

test('with the launch selected, a new waypoint goes after the takeoff, never ahead of it', async ({ app }) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.getByRole('button', { name: /^H Launch\b/ }).click();
  const drawn = await itemCount(app);
  expect(JSON.stringify(await coreCall(app, 'mission.insert', 'waypoint', 47.398, 8.546, 1)), 'with the launch selected the head inserts at index 1').toContain('"ok":true');
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBe(drawn + 1);
  const rows = app.getByRole('button', { name: /^(H|T|\d+) (Launch|Takeoff|Waypoint)\b/ });
  await expect(rows.nth(1), 'Oleg’s plan read Launch, Waypoint, Takeoff: a waypoint added after the launch went in before the takeoff').toHaveAccessibleName(/^T Takeoff\b/);
  await expect(rows.nth(2)).toHaveAccessibleName(/^1 Waypoint\b/);
});

test('the raw view of a waypoint reads in plain words and its command list changes the command', async ({ app }) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  await coreCall(app, 'advancedUi.set', true);
  await app.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await app.getByRole('button', { name: 'Show advanced settings' }).click();
  await app.getByRole('switch', { name: 'Show all values' }).click();
  await expect(app.getByText('Hold (secs)'), 'the first value is the hold, not Param1').toBeVisible();
  await expect(app.getByText('Latitude', { exact: true })).toBeVisible();
  await expect(app.getByText(/MAV_CMD_|MAV_FRAME_|^Param1$|Lat\/X/)).toHaveCount(0);
  await app.getByRole('button', { name: 'Command, Waypoint' }).click();
  await app.getByRole('menuitem', { name: 'Delay', exact: true }).click();
  await expect(app.getByText('That change was not accepted.'), 'picking a command from the list was refused for every command').toHaveCount(0);
  await expect(app.getByText('Delay added at Waypoint 3'), 'a command with no position is added at the waypoint, which stays').toBeVisible({ timeout: 15_000 });
  await expect(app.getByRole('button', { name: /^Delay\b/i }), 'the delay sits under its waypoint without a number of its own').toBeVisible();
  await expect(app.getByRole('button', { name: /^3 Waypoint\b/ })).toBeVisible();
});

test('the profile chart and the launch row read heights from the launch, not above sea level', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await expect.poll(() => itemCount(vehicle)).toBeGreaterThanOrEqual(5);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  const chart = vehicle.getByRole('img', { name: / · \d+ m/ });
  await expect(chart, 'the band of heights is read from the launch, as the waypoints are').toHaveAccessibleName(/m above launch · /, { timeout: 30_000 });
  await expect(vehicle.getByText('Height above launch (m)')).toBeVisible();
  await expect(vehicle.getByText(/^Height AMSL/)).toHaveCount(0);
  await expect(vehicle.getByRole('button', { name: /^H Launch Ground -?[\d,.]+ m AMSL/ }), 'the launch row is the ground’s own height, and says so').toBeVisible();
});

test('a plan being drawn survives a reload of the page', async ({ app }) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  const drawn = await itemCount(app);
  await app.waitForFunction(() => new Promise((resolve) => {
    const opened = indexedDB.open('keyval-store');
    opened.onsuccess = () => {
      const read = opened.result.transaction('keyval').objectStore('keyval').get('plan-draft');
      read.onsuccess = () => resolve(typeof read.result === 'string' && read.result.includes('"items"'));
      read.onerror = () => resolve(false);
    };
    opened.onerror = () => resolve(false);
  }));
  await openApp(app, { firstRun: false });
  await openMenu(app, 'Plan');
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBe(drawn);
});

test('upload sends the mission to the vehicle and offers to fly it', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await vehicle.getByRole('button', { name: 'Upload', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: 'Upload', exact: true })).toBeHidden({ timeout: 30_000 });
  await expect(vehicle.getByRole('button', { name: 'Fly', exact: true }).last()).toBeVisible();
});

test('the selected waypoint editor steps its altitude into the item list', async ({ app }) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.getByRole('button', { name: 'Waypoint 3' }).click();
  await expect(app.getByRole('button', { name: /^3 Waypoint 50\.0 m/ })).toBeVisible();
  await app.getByRole('button', { name: 'Increase Altitude' }).click();
  await app.getByRole('button', { name: 'Increase Altitude' }).click();
  await expect(app.getByRole('button', { name: /^3 Waypoint 52\.0 m/ })).toBeVisible({ timeout: 15_000 });
  await app.getByRole('slider', { name: 'Altitude' }).press('ArrowLeft');
  await expect(app.getByRole('button', { name: /^3 Waypoint 51\.0 m/ })).toBeVisible({ timeout: 15_000 });
});

test('the terrain profile charts the mission over the ground at the vehicle', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await expect.poll(() => itemCount(vehicle)).toBeGreaterThanOrEqual(5);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  const chart = vehicle.getByRole('img', { name: / · \d+ m/ });
  await expect(chart).toBeVisible({ timeout: 30_000 });
  await expect(chart).not.toHaveAccessibleName(/ground height/);
});

test('a waypoint deletes from the plan through its editor', async ({ app }) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  const drawn = await itemCount(app);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.keyboard.press('ControlOrMeta+z');
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBeLessThan(drawn);
  await app.keyboard.press('ControlOrMeta+Shift+z');
  await expect.poll(() => itemCount(app), { timeout: 30_000, message: 'an undo and redo close the burst of taps, so each delete below is its own undo step' }).toBe(drawn);
  await app.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await app.getByRole('button', { name: 'Delete waypoint 3' }).click();
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBe(drawn - 1);
  await planMenu(app, 'Undo');
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBe(drawn);
  await planMenu(app, 'Redo');
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBe(drawn - 1);
  await app.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await app.keyboard.down('Delete');
  await app.keyboard.down('Delete');
  await app.keyboard.up('Delete');
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBe(drawn - 2);
  await expect(app.getByRole('button', { name: 'Waypoint 2' }).first(), 'a held Delete removed one waypoint, not two').toBeVisible();
  await app.keyboard.press('ControlOrMeta+z');
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBe(drawn - 1);
});

test('a waypoint moves down the list and back up', async ({ app }) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await app.getByRole('button', { name: 'Increase Altitude' }).click();
  await expect(app.getByRole('button', { name: /^3 Waypoint 51\.0 m/ })).toBeVisible({ timeout: 15_000 });
  await app.getByRole('button', { name: 'Move waypoint 3 down' }).click();
  await expect(app.getByRole('button', { name: /^4 Waypoint 51\.0 m/ })).toBeVisible({ timeout: 15_000 });
  await app.getByRole('button', { name: 'Move waypoint 4 up' }).click();
  await expect(app.getByRole('button', { name: /^3 Waypoint 51\.0 m/ })).toBeVisible({ timeout: 15_000 });
  await app.getByRole('button', { name: /^1 Waypoint / }).click();
  await expect(app.getByRole('button', { name: 'Move waypoint 1 down' })).toBeVisible();
  await expect(app.getByRole('button', { name: 'Move waypoint 1 up' }), 'nothing trades places with the takeoff').toHaveCount(0);
  await app.getByRole('button', { name: /^T Takeoff/ }).click();
  await expect(app.getByRole('button', { name: /^Move takeoff/ }), 'the takeoff keeps its place').toHaveCount(0);
});

test('a typed altitude past the limit says it was capped', async ({ app }) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await app.getByRole('button', { name: /^Altitude: / }).click();
  const entry = app.getByRole('dialog').filter({ hasText: 'Altitude' });
  await entry.getByRole('textbox').fill('900');
  await expect(entry.getByText('Capped at 500 m, the highest allowed.')).toBeVisible();
  await entry.getByRole('button', { name: 'Set' }).click();
  await expect(app.getByRole('button', { name: /^3 Waypoint 500 m/ })).toBeVisible({ timeout: 15_000 });
});

test('pressing Enter in a typed altitude sets it and closes the entry', async ({ app }) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await app.getByRole('button', { name: /^Altitude: / }).click();
  const typed = app.getByRole('dialog').getByRole('textbox');
  await typed.fill('70');
  await typed.press('Enter');
  await expect(app.getByRole('button', { name: /^3 Waypoint 70\.0 m/ })).toBeVisible({ timeout: 15_000 });
  await expect(typed, 'the Enter used to land on the altitude button behind the entry and open it again').toHaveCount(0);
});

test('a new route altitude asks in plain words whether to change every waypoint, and the next waypoint takes it rather than copying the one before as QGC does', async ({ app }) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await app.getByRole('button', { name: 'Increase Altitude' }).click();
  await expect(app.getByRole('button', { name: /^3 Waypoint 51\.0 m/ })).toBeVisible({ timeout: 15_000 });
  await app.getByRole('button', { name: 'Done' }).click();
  await expect(app.getByText('New waypoints use this')).toBeVisible();
  await app.getByRole('button', { name: 'Altitude: 50 m' }).click();
  await app.getByRole('dialog').getByRole('textbox').fill('70');
  await app.getByRole('dialog').getByRole('button', { name: 'Set' }).click();
  const asked = app.getByRole('dialog').filter({ hasText: 'Change every waypoint to the new route altitude?' });
  await expect(asked).toBeVisible();
  await expect(asked.getByText(/default altitude for mission items/)).toHaveCount(0);
  await asked.getByRole('button', { name: 'Keep their altitudes' }).click();
  await expect(app.getByRole('button', { name: /^3 Waypoint 51\.0 m/ })).toBeVisible();
  await expect(app.getByRole('button', { name: 'Change all' })).toHaveCount(0);
  const drawn = await itemCount(app);
  const box = await app.getByRole('region', { name: 'Map' }).boundingBox();
  if (box === null) throw new Error('plan map is not on screen');
  await expect(async () => {
    await app.mouse.click(box.x + box.width * 0.85, box.y + box.height * 0.3);
    await expect.poll(() => itemCount(app), { timeout: 10_000 }).toBeGreaterThan(drawn);
  }).toPass({ timeout: 30_000 });
  await expect(app.getByRole('button', { name: /^\d+ Waypoint 70\.0 m/ }), 'the note says new waypoints use the route altitude, so the next one must not copy the waypoint before it').toBeVisible({ timeout: 15_000 });
});

test('switching a waypoint to AMSL keeps its height above the launch', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await expect.poll(() => itemCount(vehicle)).toBeGreaterThanOrEqual(5);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await vehicle.getByRole('button', { name: /^Relative to launch$/i }).click();
  await vehicle.getByRole('menuitem', { name: /Mixed modes/i }).click();
  const launch = vehicle.getByRole('button', { name: /Launch Ground -?[\d,.]+ m AMSL/ });
  await expect(launch).toBeVisible({ timeout: 30_000 });
  const ground = Number((await launch.innerText()).match(/Ground\s+(-?[\d,.]+) m AMSL/)?.[1]?.replace(',', ''));
  expect(ground, 'the launch has a real elevation, so the conversion is visible').toBeGreaterThan(0);
  await vehicle.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await expect(vehicle.getByRole('button', { name: /^3 Waypoint 50\.0 m/ })).toBeVisible();
  await vehicle.getByRole('button', { name: 'AMSL', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: new RegExp(`^3 Waypoint ${Math.round(ground + 50)}(\\.\\d)? m AMSL`) })).toBeVisible({ timeout: 15_000 });
  const valley = Math.round(ground - 25);
  await vehicle.getByRole('button', { name: /^Altitude: / }).click();
  const entry = vehicle.getByRole('dialog').filter({ hasText: 'Altitude' });
  await entry.getByRole('textbox').fill(`${valley}`);
  await expect(entry.getByText(/Raised to|Capped at/), 'a waypoint below the launch is a normal valley plan, not a value to raise').toHaveCount(0);
  await entry.getByRole('button', { name: 'Set' }).click();
  await expect(vehicle.getByRole('button', { name: new RegExp(`^3 Waypoint ${valley}(\\.0)? m AMSL`) })).toBeVisible({ timeout: 15_000 });
  await vehicle.getByRole('button', { name: 'Decrease Altitude' }).click();
  await expect(vehicle.getByRole('button', { name: new RegExp(`^3 Waypoint ${valley - 1}(\\.0)? m AMSL`) }), 'minus moves the waypoint down, not up to the launch').toBeVisible({ timeout: 15_000 });
  await vehicle.getByRole('button', { name: 'Relative to launch', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: /^3 Waypoint -2\d(\.\d)? m/ }), 'back relative to the launch, it stays below it').toBeVisible({ timeout: 15_000 });
});

test('a waypoint takes a hover action and gives it back', async ({ app }) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await app.getByRole('button', { name: 'Add action', exact: true }).click();
  await app.getByRole('menuitem', { name: 'Hover' }).click();
  await app.getByRole('dialog').filter({ hasText: 'Hover' }).getByRole('button', { name: 'Done' }).click();
  await expect(app.getByRole('button', { name: /^3 Waypoint .* · holds 5 s/ })).toBeVisible({ timeout: 15_000 });
  await app.getByRole('button', { name: 'Remove Hover' }).click();
  await expect(app.getByRole('button', { name: /^3 Waypoint .* · holds/ })).toHaveCount(0, { timeout: 15_000 });
});

test('after undoing a move, the selected waypoint shows its own actions, not the ones of the waypoint that moved', async ({ app }) => {
  await openMenu(app, 'Plan');
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await app.getByRole('button', { name: 'Add action', exact: true }).click();
  await app.getByRole('menuitem', { name: 'Photo' }).click();
  await app.getByRole('dialog').filter({ hasText: 'Take photo' }).getByRole('button', { name: 'Done' }).click();
  await expect(app.getByRole('button', { name: 'Take photo', exact: true })).toBeVisible();
  await app.getByRole('button', { name: 'Move waypoint 3 up' }).click();
  await expect(app.getByRole('button', { name: 'Delete waypoint 2' }), 'the moved waypoint is now the second').toBeVisible({ timeout: 15_000 });
  await expect(app.getByRole('button', { name: 'Take photo', exact: true })).toBeVisible();
  await app.getByRole('button', { name: 'Undo', exact: true }).click();
  await expect(app.getByRole('button', { name: /^2 Waypoint / })).toBeVisible({ timeout: 15_000 });
  await expect(app.getByRole('button', { name: 'Delete waypoint 2' })).toBeVisible();
  await expect(app.getByRole('button', { name: 'Add action', exact: true })).toBeVisible();
  await expect(app.getByRole('button', { name: 'Take photo', exact: true }), 'waypoint 2 has no photo; the panel kept the moved waypoint’s camera').toHaveCount(0);
});

test('a new fixed-wing landing says how to finish its setup, and finishing it clears the upload block', async ({ app }) => {
  await connectMock(app, [], 'APM ArduPlane Vehicle');
  await openPlanAtVehicle(app);
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.getByRole('button', { name: 'Land', exact: true }).click();
  const unfinished = app.getByRole('button', { name: /^\S+ Landing pattern .*Finish the landing setup/ });
  await expect(unfinished).toBeVisible({ timeout: 15_000 });
  await expect(app.getByText(/^Drag the loiter point/), 'the editor offered no way to finish the setup the upload asked for').toBeVisible();
  await app.getByRole('button', { name: 'Done', exact: true }).last().click();
  await expect(unfinished).toHaveCount(0, { timeout: 15_000 });
});

test('an AMSL plan read back from the aircraft keeps its AMSL frame instead of showing Mixed modes', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openPlanAtVehicle(app);
  await app.getByRole('button', { name: 'Relative to launch', exact: true }).click();
  await app.getByRole('menuitem', { name: /^AMSL/ }).click();
  await expect(app.getByRole('button', { name: 'AMSL', exact: true })).toBeVisible();
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.getByRole('button', { name: 'Upload', exact: true }).click();
  await expect(app.getByRole('button', { name: 'Upload', exact: true })).toBeHidden({ timeout: 30_000 });
  await planMenu(app, 'Download from vehicle');
  await expect(app.getByText('Plan downloaded from the vehicle.')).toBeVisible({ timeout: 30_000 });
  await expect(app.getByRole('button', { name: 'AMSL', exact: true }), 'the takeoff climbs relative to launch, which made the read-back plan Mixed modes').toBeVisible();
  await expect(app.getByRole('button', { name: 'Mixed modes', exact: true })).toHaveCount(0);
});

test('a plan the connected vehicle cannot fly says which items, and is not converted', async ({ app }) => {
  await openMenu(app, 'Plan');
  await app.getByRole('button', { name: /^Vehicle, / }).click();
  await app.getByRole('menuitem', { name: 'VTOL' }).click();
  await expect(app.getByText(/For .*VTOL/)).toBeVisible();
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.addLocatorHandler(app.getByRole('dialog').filter({ hasText: 'Aircraft connected' }), (prompt) => prompt.getByRole('button', { name: 'Keep my route' }).click());
  await app.getByRole('navigation').getByRole('button', { name: 'Fly', exact: true }).click();
  await connectMock(app);
  await openMenu(app, 'Plan');
  await app.getByRole('button', { name: 'Upload', exact: true }).click();
  const far = app.getByRole('dialog').filter({ hasText: 'from the aircraft' });
  const mismatch = app.getByRole('dialog').filter({ hasText: 'different firmware' });
  await expect(far.or(mismatch)).toBeVisible();
  if (await far.isVisible()) await far.getByRole('button', { name: 'Upload anyway' }).click();
  await mismatch.getByRole('button', { name: 'Convert to this vehicle' }).click();
  await expect(app.getByText(/^The connected vehicle cannot fly item 1 \(VTOL takeoff\)/)).toBeVisible();
  await expect(app.getByText('Plan converted to this vehicle. Check it, then upload.')).toHaveCount(0);
  await app.getByRole('button', { name: 'Upload', exact: true }).click();
  if (await far.isVisible()) await far.getByRole('button', { name: 'Upload anyway' }).click();
  await expect(mismatch, 'the plan still holds the VTOL takeoff, so it still differs from the vehicle').toBeVisible();
});

test('a plan made for another firmware converts to the connected vehicle', async ({ app }) => {
  await openMenu(app, 'Plan');
  await app.getByRole('radiogroup', { name: 'Firmware' }).getByRole('radio', { name: 'ArduPilot' }).click();
  await expect(app.getByText(/For ArduPilot/)).toBeVisible();
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.addLocatorHandler(app.getByRole('dialog').filter({ hasText: 'Aircraft connected' }), (prompt) => prompt.getByRole('button', { name: 'Keep my route' }).click());
  await app.getByRole('navigation').getByRole('button', { name: 'Fly', exact: true }).click();
  await connectMock(app);
  await openMenu(app, 'Plan');
  await app.getByRole('button', { name: 'Upload', exact: true }).click();
  const far = app.getByRole('dialog').filter({ hasText: 'from the aircraft' });
  const mismatch = app.getByRole('dialog').filter({ hasText: 'different firmware' });
  await expect(far.or(mismatch)).toBeVisible();
  if (await far.isVisible()) await far.getByRole('button', { name: 'Upload anyway' }).click();
  await mismatch.getByRole('button', { name: 'Convert to this vehicle' }).click();
  await expect(app.getByText('Plan converted to this vehicle. Check it, then upload.')).toBeVisible();
  await expect(app.getByText(/For PX4/)).toBeVisible({ timeout: 15_000 });
});

test('a PX4 survey plan converts for ArduCopter without asking to remove the camera mode it cannot reach', async ({ app }) => {
  await openMenu(app, 'Plan');
  await app.getByRole('button', { name: 'Center', exact: true }).click();
  await app.getByRole('menuitem', { name: 'Coordinates…' }).click();
  const centre = app.getByRole('dialog').filter({ hasText: 'Center map on coordinate' });
  await centre.getByRole('textbox', { name: 'Latitude' }).fill('47.3977');
  await centre.getByRole('textbox', { name: 'Longitude' }).fill('8.5456');
  await centre.getByRole('button', { name: 'Center' }).click();
  await expect(app.locator('span.text-label-small').getByText(/^\d+ (m|km)$/)).toHaveText(/^\d+ m$/, { timeout: 30_000 });
  await expect(app.getByText(/For PX4/)).toBeVisible();
  await app.getByRole('button', { name: 'Pattern', exact: true }).click();
  await app.getByRole('dialog').filter({ hasText: 'Which pattern?' }).getByRole('button', { name: 'Survey', exact: true }).click();
  await expect(app.getByText('AREA', { exact: true })).toBeVisible();
  await app.addLocatorHandler(app.getByRole('dialog').filter({ hasText: 'Aircraft connected' }), (prompt) => prompt.getByRole('button', { name: 'Keep my route' }).click());
  await app.getByRole('navigation').getByRole('button', { name: 'Fly', exact: true }).click();
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openMenu(app, 'Plan');
  await app.getByRole('button', { name: 'Upload', exact: true }).click();
  const far = app.getByRole('dialog').filter({ hasText: 'from the aircraft' });
  const mismatch = app.getByRole('dialog').filter({ hasText: 'different firmware' });
  await expect(far.or(mismatch)).toBeVisible();
  if (await far.isVisible()) await far.getByRole('button', { name: 'Upload anyway' }).click();
  await mismatch.getByRole('button', { name: 'Convert to this vehicle' }).click();
  await expect(app.getByText('Plan converted to this vehicle. Check it, then upload.'), 'the survey set a PX4 camera mode at the mission start, which no list row lets an operator remove').toBeVisible();
});

test.describe('on a phone', () => {
  test.use({ viewport: { width: 412, height: 860 } });

  test('the plan tools sit above the open sheet, where a tap reaches them', async ({ vehicle }) => {
    await openPlanAtVehicle(vehicle);
    await vehicle.getByRole('button', { name: 'Waypoint', exact: true }).click();
    await expect(vehicle.getByText('Waypoint 1', { exact: true }), 'the open waypoint editor fills the sheet').toBeVisible();
    const uncovered = async (name: string) => {
      const tool = vehicle.getByRole('button', { name, exact: true });
      await expect(tool).toBeInViewport({ ratio: 1 });
      return tool.evaluate((button) => {
        const box = button.getBoundingClientRect();
        return button.contains(document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2));
      });
    };
    await Promise.all(['Pattern', 'Return', 'Drop'].map(async (name) => expect(await uncovered(name), `${name} was scrolled away inside the rail under the sheet`).toBe(true)));
  });
});

const planRows = (page: Page) =>
  page.evaluate(async (module) => {
    const view = (await ((await import(module)) as { Qgc: { get: (path: string) => Promise<unknown> } }).Qgc.get('view.missionItems(geometry)')) as { items?: { gimbalPitch?: number | null; gimbalYaw?: number | null }[] } | null;
    return view?.items ?? [];
  }, '/src/bridge/Qgc.ts');

test('a waypoint that tilts the camera tells the map how far down it looks, so its wedge can be drawn to match', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openPlanAtVehicle(app);
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  await app.getByRole('button', { name: 'Waypoint 3' }).first().click();
  await app.getByRole('button', { name: 'Add action', exact: true }).click();
  await app.getByRole('menuitem', { name: 'Tilt camera' }).click();
  const editor = app.getByRole('dialog').filter({ hasText: 'Pitch' });
  await editor.getByRole('button', { name: /^Pitch: / }).click();
  await app.getByRole('textbox', { name: /^Pitch/ }).fill('-60');
  await app.getByRole('button', { name: 'Set', exact: true }).click();
  await editor.getByRole('button', { name: 'Done' }).click();
  await expect.poll(async () => (await planRows(app)).map((it) => it.gimbalPitch ?? null).filter((it) => it !== null), { timeout: 30_000 }).toContain(-60);
});

test('a right-click on the plan map asks before it adds, and adds the waypoint at the end of the route', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openPlanAtVehicle(app);
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  const before = await itemCount(app);
  const box = (await app.getByRole('region', { name: 'Map' }).boundingBox())!;
  await app.mouse.click(box.x + box.width * 0.75, box.y + box.height * 0.85, { button: 'right' });
  await expect(app.getByRole('menuitem', { name: 'Set launch here' })).toBeVisible();
  expect(await itemCount(app), 'nothing is added until the pilot picks it').toBe(before);
  await app.getByRole('menuitem', { name: 'Add waypoint here' }).click();
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBe(before + 1);
  await expect(app.getByRole('button', { name: `Waypoint ${before}` }).first(), 'it lands last, after the route already drawn: the takeoff takes no number, so it is waypoint as many as there were items').toBeVisible();
});

const LONG_PRESS_HAS_PASSED_MS = 1_000;

test('a long press on the plan map opens the menu and it stays open when the finger lifts, but a drag or a second finger opens nothing', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openPlanAtVehicle(app);
  await tapPlanMission(app);
  await expect.poll(() => itemCount(app)).toBeGreaterThanOrEqual(5);
  await app.getByRole('button', { name: 'Done' }).click();
  const before = await itemCount(app);
  const box = (await app.getByRole('region', { name: 'Map' }).boundingBox())!;
  const at = { x: Math.round(box.x + box.width * 0.75), y: Math.round(box.y + box.height * 0.85) };
  const finger = await app.context().newCDPSession(app);
  const touch = (type: 'touchStart' | 'touchMove' | 'touchEnd', points: readonly { x: number; y: number }[]) => finger.send('Input.dispatchTouchEvent', { type, touchPoints: [...points] });
  const menu = app.getByRole('menuitem', { name: 'Add waypoint here' });
  const held = async () => {
    const started = Date.now();
    await expect.poll(() => Date.now() - started, { timeout: LONG_PRESS_HAS_PASSED_MS * 3 }).toBeGreaterThan(LONG_PRESS_HAS_PASSED_MS);
  };

  await touch('touchStart', [at]);
  await touch('touchMove', [{ x: at.x - 60, y: at.y - 60 }]);
  await held();
  await touch('touchEnd', []);
  await expect(menu, 'a finger that moved was panning the map').toHaveCount(0);

  await touch('touchStart', [at]);
  await touch('touchStart', [at, { x: at.x - 80, y: at.y - 80 }]);
  await held();
  await touch('touchEnd', []);
  await expect(menu, 'two fingers are a pinch').toHaveCount(0);
  expect(await itemCount(app), 'neither gesture added a waypoint').toBe(before);

  await touch('touchStart', [at]);
  await expect(menu, 'the menu opens under the held finger').toBeVisible();
  await touch('touchEnd', []);
  await held();
  await expect(menu, 'lifting the finger leaves the menu open to be read').toBeVisible();
  await menu.click();
  await expect.poll(() => itemCount(app), { timeout: 30_000 }).toBe(before + 1);
});

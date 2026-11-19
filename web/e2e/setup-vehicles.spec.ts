import { connectMock, expect, openAircraftSetup, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('a browser leaves Remote support out of ArduPilot setup, since it forwards over a UDP socket', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await expect(app.getByRole('button', { name: /^Logging / })).toBeVisible();
  await expect(app.getByRole('button', { name: /^Remote support/ })).toHaveCount(0);
});

test('moving a compass to priority 2 swaps it with the compass there instead of listing it twice', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Sensors/ }).first().click();
  await app.getByRole('button', { name: 'Sensor settings' }).click();
  const settings = app.getByRole('dialog').filter({ hasText: 'Sensor Settings' });
  await settings.getByRole('button', { name: 'Priority 1' }).click();
  await app.getByRole('menuitem', { name: 'Priority 2' }).click();
  await app.getByRole('dialog').filter({ hasText: 'Reboot vehicle for changes to take effect.' }).getByRole('button', { name: 'Cancel' }).click();
  await expect.poll(async () => (await settings.innerText()).replace(/\s+/g, ' ')).toMatch(/Compass 1 .*Priority 2 .*Compass 2 .*Priority 1 .*Compass 3 .*Priority 3/);
  await expect(settings).not.toContainText('Not Set');
});

test('leaving Sensors in the middle of a compass calibration asks to finish or cancel it first', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Sensors/ }).first().click();
  await app.getByRole('button', { name: /^Compass/ }).first().click();
  await app.getByRole('dialog').getByRole('button', { name: 'OK' }).click();
  await expect(app.getByText('Calibrating Compass', { exact: true })).toBeVisible();
  await app.getByRole('button', { name: /^Radio / }).click();
  await expect(app.getByText('Complete or cancel the current calibration first')).toBeVisible();
});

test('a sub opens Motors without an Unable to send command popup', async ({ app }) => {
  await connectMock(app, [], 'APM ArduSub Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Motors / }).click();
  await expect(app.getByText('Automatic motor direction detection', { exact: true })).toBeVisible();
  await expect(app.getByText(/Unable to send command/)).toHaveCount(0);
});

test('a sub gathers its failsafes on Failsafes and heads its safety rows in plain words', async ({ app }) => {
  await connectMock(app, [], 'APM ArduSub Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Failsafes / }).click();
  await expect(app.getByText('Leak failsafe', { exact: true })).toBeVisible();
  await expect(app.getByText('Internal pressure failsafe', { exact: true })).toBeVisible();
  await app.getByRole('button', { name: /^Flight safety / }).click();
  await expect(app.getByText('Arming checks to skip', { exact: true })).toBeVisible();
  await expect(app.getByText(/\(bitmask\)/)).toHaveCount(0);
  await expect(app.getByText('Leak failsafe', { exact: true })).toHaveCount(0);
});

test('a sub frame change can be rebooted from the Frame page after its prompt was dismissed', async ({ app }) => {
  await connectMock(app, [], 'APM ArduSub Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Frame / }).click();
  await app.getByRole('button', { name: 'Vectored-6DOF Vectored-6DOF' }).click();
  await app.getByRole('dialog').filter({ hasText: 'Reboot vehicle for changes to take effect.' }).getByRole('button', { name: 'Cancel' }).click();
  await expect(app.getByText('The new frame takes effect after the vehicle reboots.', { exact: true })).toBeVisible();
  await expect(app.getByRole('button', { name: 'Reboot vehicle', exact: true })).toBeVisible();
});

test('a plane offers its flight modes and failsafe actions in plain words, without QuadPlane modes', async ({ app }) => {
  await connectMock(app, [], 'APM ArduPlane Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Flight modes / }).click();
  await app.getByRole('button', { name: /^Flight mode 1, / }).click();
  await expect(app.getByRole('menuitem', { name: 'FBW A', exact: true })).toBeVisible();
  await expect(app.getByRole('menuitem', { name: 'Stabilize', exact: true })).toBeVisible();
  await expect(app.getByRole('menuitem', { name: /^(QuadPlane|QSTABILIZE|QHOVER|STABILIZE$|FBWA$)/ })).toHaveCount(0);
  await app.keyboard.press('Escape');
  await app.getByRole('button', { name: /^Failsafes / }).click();
  await app.getByRole('button', { name: /^Short failsafe action, / }).click();
  await expect(app.getByRole('menuitem', { name: 'Circle, or carry on if on a mission, guided or loitering', exact: true })).toBeVisible();
  await expect(app.getByRole('menuitem', { name: /\|/ })).toHaveCount(0);
});

test('a plane offers no CompassMot, which only ArduCopter runs', async ({ app }) => {
  await connectMock(app, [], 'APM ArduPlane Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Sensors/ }).first().click();
  await expect(app.getByRole('button', { name: /^Compass/ }).first()).toBeVisible();
  await expect(app.getByRole('button', { name: /^CompassMot/ })).toHaveCount(0);
});

test('a plane says why it has no motor test instead of offering eight motors', async ({ app }) => {
  await connectMock(app, [], 'APM ArduPlane Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Motors / }).click();
  await expect(app.getByText(/^ArduPilot only spins VTOL lift motors in a motor test\./)).toBeVisible();
  await expect(app.getByText('Warning: Unable to determine motor count')).toHaveCount(0);
});

test('a rover tests its throttle and steering instead of eight unknown motors', async ({ app }) => {
  await connectMock(app, [], 'APM ArduRover Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Motors / }).click();
  await expect(app.getByRole('button', { name: /^Throttle/ })).toBeVisible();
  await expect(app.getByRole('button', { name: /^Steering/ })).toBeVisible();
  await expect(app.getByText('Warning: Unable to determine motor count')).toHaveCount(0);
  await expect(app.getByRole('button', { name: /^Motor H/ })).toHaveCount(0);
});

test('a plane refuses a cruise airspeed above its maximum', async ({ app }) => {
  await connectMock(app, [], 'APM ArduPlane Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Airspeed / }).click();
  const cruise = app.getByRole('textbox', { name: 'Cruise airspeed' });
  await cruise.fill('35');
  await cruise.press('Enter');
  await expect(app.getByText('Cruise airspeed cannot be above the maximum airspeed (30).')).toBeVisible();
  await expect(app.getByRole('button', { name: /^Airspeed .*Cruise airspeed 22/ })).toBeVisible();
});

test('a servo minimum above its trim is refused, and a kept change offers Undo', async ({ app }) => {
  await connectMock(app, [], 'APM ArduPlane Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Servo outputs / }).click();
  const minimum = app.getByRole('textbox', { name: 'Min, SERVO1_MIN' });
  await minimum.fill('2000');
  await minimum.press('Enter');
  await expect(app.getByText('Servo 1 minimum cannot be above its trim (1500).')).toBeVisible();
  await minimum.fill('1050');
  await minimum.press('Enter');
  const notice = app.getByRole('status').filter({ hasText: 'Servo 1 min: 1050' });
  await expect(notice).toBeVisible();
  await notice.getByRole('button', { name: 'Undo' }).click();
  await expect(minimum).toHaveValue('1100');
});

test('a PX4 vehicle on MAVLink-only control reads its radio and mode switch as not used, and says why on Radio', async ({ app }) => {
  await connectMock(app);
  await openAircraftSetup(app);
  await expect(app.getByRole('button', { name: /^Radio Not used: MAVLink only/ })).toBeVisible();
  await expect(app.getByRole('button', { name: /^Flight modes Not used: MAVLink only/ })).toBeVisible();
  await app.getByRole('button', { name: /^Radio / }).click();
  await expect(app.getByText(/^This vehicle takes manual control over MAVLink only/)).toBeVisible();
});

test('PX4 safety names the geofence action as Settings does and lists each limit once', async ({ app }) => {
  await connectMock(app);
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Safety / }).click();
  await expect(app.getByRole('button', { name: /^Geofence violation action, Hover/ })).toBeVisible();
  await expect(app.getByText('Max altitude', { exact: true })).toHaveCount(1);
  await expect(app.getByRole('switch', { name: 'Max radius' })).toHaveCount(0);
  await expect(app.getByText('-1.000 m')).toHaveCount(0);
});

test('an open Setup page stays open when the window narrows to a phone', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Power / }).click();
  await expect(app.getByText('Battery capacity').first()).toBeVisible();
  await app.setViewportSize({ width: 412, height: 860 });
  await expect(app.getByText('Battery capacity').first()).toBeVisible();
  await app.getByRole('button', { name: 'Back' }).first().click();
  await expect(app.getByRole('button', { name: /^Sensors/ })).toBeVisible();
});

test('the copter frame tiles keep their names inside the tile', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  const tile = app.getByRole('button', { name: /^6DoF Scripting/ });
  const label = tile.getByText('6DoF Scripting', { exact: true });
  const [outer, inner] = await Promise.all([tile.boundingBox(), label.boundingBox()]);
  expect(inner !== null && outer !== null && inner.x >= outer.x && inner.x + inner.width <= outer.x + outer.width + 1).toBe(true);
  await expect(app.getByRole('button', { name: /^Heli Dual/ })).toBeVisible();
});

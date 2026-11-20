import { connectFreshMock, connectMock, expect, openAircraftSetup, openMenu, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

const PAGES = ['Failsafes', 'Flight modes', 'Flight safety', 'Gimbal', 'Joystick', 'Logging', 'Motors', 'Power', 'Radio', 'Scripting', 'Sensors', 'Servo outputs', 'Tuning'] as const;

PAGES.map((page) =>
  test(`ArduPilot setup › ${page} opens`, async ({ app }) => {
    await connectMock(app, [], 'APM ArduCopter Vehicle');
    await openAircraftSetup(app);
    const titled = app.getByText(page, { exact: true });
    const before = await titled.count();
    await app.getByRole('button', { name: new RegExp(`^${page} `) }).first().click();
    await expect.poll(() => titled.count()).toBeGreaterThan(before);
  }),
);

test('ArduPilot setup opens on the Frame page, the first step of setting up', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await expect(app.getByText(/^Currently set to frame class/)).toBeVisible();
});

test('ArduPilot setup opens on the ESC page with its calibration', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^ESC / }).click();
  await expect(app.getByRole('button', { name: 'Calibrate', exact: true }).first()).toBeVisible();
});

test('ArduPilot flight modes setup changes a mode slot', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Flight modes / }).click();
  const slot = app.getByText('Flight mode 1', { exact: true }).locator('xpath=ancestor::div[.//button][1]');
  const choice = slot.getByRole('button').first();
  await expect(choice).not.toHaveAccessibleName('');
  await choice.click();
  await app.getByRole('menuitem', { name: 'Loiter', exact: true }).or(app.getByRole('option', { name: 'Loiter', exact: true })).first().click();
  await expect(choice).toHaveAccessibleName('Flight mode 1, Loiter');
  await expect(slot.getByRole('textbox')).toHaveCount(0);
});

test('ArduPilot compass calibration runs to its reboot reminder', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Sensors/ }).first().click();
  await app.getByRole('button', { name: /^Compass/ }).first().click();
  await app.getByRole('dialog').getByRole('button', { name: 'OK' }).click();
  await expect(app.getByText(/Reboot the vehicle so the new calibration takes effect/)).toBeVisible({ timeout: 60_000 });
});

test('ArduPilot flight safety links the return altitude to Settings › Safety instead of a second editor', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Flight safety / }).click();
  const link = app.getByRole('button', { name: /^Return-to-home altitude In Settings › Safety/ });
  await expect(link).toContainText('15 m');
  await expect(app.getByRole('textbox', { name: /RTL altitude|Return altitude/i })).toHaveCount(0);
  await link.click();
  await expect(app.getByRole('slider', { name: 'Return-to-home altitude' })).toBeVisible();
  await expect(app.getByText('Climbs to this height before flying home.', { exact: true })).toBeInViewport();
});

test('ArduPilot failsafes open Settings › Safety on its own page, not on a blank or General one', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Failsafes / }).click();
  await app.getByRole('button', { name: /^Low battery In Settings › Safety/ }).click();
  await expect(app.getByText('When the battery runs low.', { exact: true })).toBeInViewport();
});

test('ArduPilot failsafes link both batteries to Settings › Safety instead of editing battery 2 inline', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Failsafes / }).click();
  await expect(app.getByRole('button', { name: /^Critical battery In Settings › Safety/ })).toBeVisible();
  await expect(app.getByRole('button', { name: /^Battery 2 critical In Settings › Safety/ })).toBeVisible();
  await expect(app.getByRole('button', { name: /^Low battery failsafe action, / })).toHaveCount(0);
  await app.getByRole('button', { name: /^Battery 2 low In Settings › Safety/ }).click();
  await expect(app.getByText('When battery 2 runs low.', { exact: true })).toBeInViewport();
});

test('ArduPilot ESC calibration says what happens before it arms, and can be taken back', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^ESC / }).click();
  const calibrate = app.getByRole('button', { name: 'Calibrate', exact: true }).first();
  await calibrate.click();
  const confirm = app.getByRole('dialog').filter({ hasText: 'Calibrate the ESCs?' });
  await expect(confirm).toContainText('Take the propellers off first.');
  await confirm.getByRole('button', { name: 'Cancel' }).click();
  await expect(app.getByText(/^Click Calibrate to start, then/)).toBeVisible();
  await calibrate.click();
  await confirm.getByRole('button', { name: 'Calibrate', exact: true }).click();
  await expect(app.getByText(/^Now perform these steps/)).toBeVisible();
  await app.getByRole('button', { name: 'Cancel calibration', exact: true }).click();
  await expect(app.getByText(/^Click Calibrate to start, then/)).toBeVisible();
  await expect(calibrate).toBeEnabled();
});

test('an ArduPilot accelerometer calibration keeps the pilot on Sensors, since it cannot be cancelled', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Sensors/ }).first().click();
  await app.getByRole('button', { name: /^Accelerometer/ }).first().click();
  await app.getByRole('dialog').getByRole('button', { name: 'OK' }).click();
  await expect(app.getByText('1 of 6 positions', { exact: true })).toBeVisible({ timeout: 30_000 });
  await app.getByRole('button', { name: 'Next', exact: true }).click();
  await expect(app.getByText('Place the vehicle on its left side, hold it still and press Next', { exact: true })).toBeVisible();
  await app.getByRole('button', { name: /^Radio / }).click();
  await expect(app.getByText(/Finish this calibration first/)).toBeVisible();
  await expect(app.getByText('2 of 6 positions', { exact: true })).toBeVisible();
  await expect(app.getByText('Calibration failed. Calibration log will be displayed.')).toHaveCount(0);
});

test('an ArduPilot frame class is picked with no reboot prompt over the page, so its frame type is chosen before the page reboots', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await expect(app.getByText(/^Currently set to frame class 'Quad' and frame type 'Plus'/)).toBeVisible();
  await app.getByRole('button', { name: 'Hexa Hexa' }).click();
  await expect(app.getByText(/^Currently set to frame class 'Hexa' and frame type 'Plus'/), 'the frame type is kept').toBeVisible();
  await app.getByRole('button', { name: 'X', exact: true }).click({ timeout: 5_000 });
  await expect(app.getByText(/^Currently set to frame class 'Hexa' and frame type 'X'/)).toBeVisible();
  await expect(app.getByRole('dialog').filter({ hasText: 'Reboot vehicle for changes to take effect.' }), 'a reboot prompt over the page took the frame type before it could be chosen').toHaveCount(0);
  await expect(app.getByRole('button', { name: 'Reboot vehicle', exact: true })).toBeVisible();
});

test('the ArduPilot radio calibration start can be backed out of', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Radio / }).click();
  await expect(app.getByText(/ carrying a signal\.$/)).toBeVisible();
  await app.getByRole('button', { name: 'Calibrate', exact: true }).click();
  const zero = app.getByRole('dialog').filter({ hasText: 'Zero Trims' });
  await zero.getByRole('button', { name: 'Cancel' }).click();
  await expect(zero).toHaveCount(0);
  await expect(app.getByRole('button', { name: 'Calibrate', exact: true })).toBeVisible();
  await expect(app.getByRole('button', { name: 'Next', exact: true })).toHaveCount(0);
});

test('PX4 airframe and PID tuning open here instead of pointing at a desktop app', async ({ app }) => {
  await connectMock(app);
  await openAircraftSetup(app);
  await expect(app.getByText('On desktop', { exact: true })).toHaveCount(0);
  await app.getByRole('button', { name: /^Airframe / }).click();
  await expect(app.getByRole('button', { name: 'Apply and Restart' })).toBeVisible();
  await app.getByRole('button', { name: /^PID tuning / }).click();
  await expect(app.getByText('Use manual tuning', { exact: true })).toBeVisible();
});

test('ArduPilot setup lists its pages in the order an aircraft is set up', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  const top = async (name: string) => {
    const box = await app.getByRole('button', { name: new RegExp(`^${name} `) }).first().boundingBox();
    if (box === null) throw new Error(`${name} is not listed`);
    return box.y;
  };
  const order = ['Frame', 'Sensors', 'Radio', 'Flight modes', 'Power', 'Failsafes', 'Flight safety', 'ESC'];
  await expect(app.getByRole('button', { name: /^ESC / })).toBeVisible();
  const tops = await Promise.all(order.map(top));
  expect(tops).toEqual(tops.toSorted((a, b) => a - b));
});

test('Setup flashes firmware with or without a vehicle', async ({ app }) => {
  await openMenu(app, 'Setup');
  await app.getByRole('button', { name: 'Flash firmware' }).click();
  await expect(app.getByText(/^(No USB serial devices attached|Flash a board over USB|Flash the flight controller)/)).toBeVisible();
  await openMenu(app, 'Fly');
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Firmware Flash a board/ }).click();
  await expect(app.getByText(/^(No USB serial devices attached|Flash a board over USB|Flash the flight controller)/)).toBeVisible();
});

test('after the accelerometer the Sensors row names the compass still to calibrate instead of reading Ready', async ({ app }) => {
  const prompt = await connectFreshMock(app, 'APM ArduCopter Vehicle');
  await prompt.getByRole('button', { name: 'Open Setup', exact: true }).click();
  await app.getByRole('button', { name: 'Quad Quad' }).click();
  const sensors = app.getByRole('button', { name: /^Sensors / }).first();
  await sensors.click();
  await app.getByRole('button', { name: /^Accelerometer/ }).first().click();
  await app.getByRole('dialog').getByRole('button', { name: 'OK' }).click();
  await [1, 2, 3, 4, 5, 6].reduce(
    (done, step) =>
      done.then(async () => {
        await expect(app.getByText(`${step} of 6 positions`, { exact: true })).toBeVisible({ timeout: 30_000 });
        await app.getByRole('button', { name: 'Next', exact: true }).click();
      }),
    Promise.resolve(),
  );
  await app.getByRole('dialog').filter({ hasText: 'Calibration complete' }).getByRole('button', { name: 'Close' }).click();
  await expect(app.getByRole('button', { name: /^Accelerometer Calibrated/ })).toBeVisible();
  await expect(sensors, 'the row read Ready over a compass that was never calibrated').toHaveAccessibleName(/^Sensors Compass setup required/);
});

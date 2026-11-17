import { connectMock, expect, openAircraftSetup, openMenu, test } from './fixtures';

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
  await expect(choice).toHaveAccessibleName('Loiter');
  await expect(slot.getByRole('textbox')).toHaveCount(0);
});

test('ArduPilot compass calibration runs to its reboot reminder', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Sensors/ }).first().click();
  await app.getByRole('button', { name: /^Compass/ }).first().click();
  await app.getByRole('dialog').getByRole('button', { name: 'OK' }).click();
  await expect(app.getByText(/YOU MUST REBOOT YOUR VEHICLE AFTER EACH CALIBRATION/)).toBeVisible({ timeout: 60_000 });
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
});

test('ArduPilot setup lists its pages in the order an aircraft is set up', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  const top = async (name: string) => (await app.getByRole('button', { name: new RegExp(`^${name} `) }).first().boundingBox())?.y ?? -1;
  const order = ['Frame', 'Sensors', 'Radio', 'Flight modes', 'Power', 'Failsafes', 'Flight safety', 'ESC'];
  await expect(app.getByRole('button', { name: /^ESC / })).toBeVisible();
  const tops = await Promise.all(order.map(top));
  expect(tops).toEqual(tops.toSorted((a, b) => a - b));
});

test('Aircraft setup flashes firmware with or without a vehicle', async ({ app }) => {
  await openMenu(app, 'Aircraft setup');
  await app.getByRole('button', { name: 'Flash firmware' }).click();
  await expect(app.getByText(/^(No USB serial devices attached|Flash a board over USB|Flash the flight controller)/)).toBeVisible();
  await openMenu(app, 'Fly');
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Firmware Flash a board/ }).click();
  await expect(app.getByText(/^(No USB serial devices attached|Flash a board over USB|Flash the flight controller)/)).toBeVisible();
});

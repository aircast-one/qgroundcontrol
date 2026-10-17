import { connectMock, expect, openAircraftSetup, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

const PAGES = ['Failsafes', 'Flight modes', 'Flight safety', 'Frame', 'Gimbal', 'Joystick', 'Logging', 'Motors', 'Power', 'Radio', 'Scripting', 'Sensors', 'Servo outputs', 'Tuning'] as const;

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

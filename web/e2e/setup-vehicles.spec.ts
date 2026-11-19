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

test('a sub opens Motors without an Unable to send command popup', async ({ app }) => {
  await connectMock(app, [], 'APM ArduSub Vehicle');
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Motors / }).click();
  await expect(app.getByText('Automatic motor direction detection', { exact: true })).toBeVisible();
  await expect(app.getByText(/Unable to send command/)).toHaveCount(0);
});

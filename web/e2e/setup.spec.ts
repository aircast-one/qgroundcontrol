import { connectMock, expect, openAircraftSetup, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('ArduPilot setup names the frame and failsafe actions from the parameter metadata', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await expect(app.getByRole('button', { name: /^Frame Quad/ })).toBeVisible();
  await expect(app.getByRole('button', { name: /^Failsafes .*RTL/ })).toBeVisible();
  await expect(app.getByRole('button', { name: /^Flight modes .*Stabilize/ })).toBeVisible();
});

test('ArduPilot follow takes another connected vehicle as its lead', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await app.getByRole('button', { name: 'Settings', exact: true }).click();
  await app.getByRole('tablist').getByRole('tab', { name: 'Transmission' }).click();
  await app.getByRole('button', { name: /^Links / }).click();
  await app.getByRole('button', { name: 'Add link…' }).click();
  await app.getByRole('button', { name: /^Simulated vehicle/ }).click();
  await app.getByRole('button', { name: 'Vehicle Type' }).click();
  await app.getByRole('menuitem', { name: 'APM ArduCopter Vehicle' }).click();
  await app.getByRole('button', { name: 'Start', exact: true }).click();
  await app.getByRole('button', { name: 'Close settings' }).click();
  await openAircraftSetup(app);
  await app.getByRole('button', { name: /^Follow me/ }).click();
  await app.getByRole('checkbox', { name: 'Enable follow me' }).click();
  await expect(app.getByRole('checkbox', { name: 'Enable follow me' })).toBeChecked();
  const reset = app.getByRole('button', { name: 'Reset to supported settings' });
  const settings = app.getByText('Follow me settings');
  await expect(reset.or(settings)).toBeVisible();
  if (await reset.isVisible()) await reset.click();
  await app.getByRole('button', { name: 'Ground station', exact: true }).click();
  await app.getByRole('menuitem', { name: 'Vehicle 129' }).click();
  await expect(app.getByRole('button', { name: 'Vehicle 129', exact: true })).toBeVisible();
  await expect(app.getByRole('button', { name: 'Point at Vehicle 129', exact: true })).toBeVisible();
});

test('PX4 setup lists sensors, radio and parameters for the vehicle', async ({ app }) => {
  await connectMock(app);
  await openAircraftSetup(app);
  await expect(app.getByRole('button', { name: /^Sensors/ })).toBeVisible();
  await expect(app.getByRole('button', { name: /^Radio/ })).toBeVisible();
});

test('the motor test stays locked until the propellers-off switch, then takes a throttle', async ({ vehicle }) => {
  await openAircraftSetup(vehicle);
  await vehicle.getByRole('button', { name: /^Motors/ }).click();
  await expect(vehicle.getByRole('button', { name: 'Motor 1 Locked' })).toBeDisabled();
  await expect(vehicle.getByRole('slider', { name: 'Throttle' })).toBeDisabled();
  await vehicle.getByRole('switch', { name: /Propellers are removed/ }).click();
  await expect(vehicle.getByRole('button', { name: 'Motor 1 Click to spin' })).toBeEnabled();
  const throttle = vehicle.getByRole('slider', { name: 'Throttle' });
  await throttle.press('ArrowRight');
  await throttle.press('ArrowRight');
  await throttle.press('ArrowRight');
  await expect(throttle).toHaveAttribute('aria-valuenow', '3');
  await expect(vehicle.getByText('3%', { exact: true })).toBeVisible();
  await vehicle.getByRole('button', { name: 'Motor 1 Click to spin' }).click();
  await vehicle.getByRole('button', { name: 'Stop' }).click();
  await expect(vehicle.getByRole('button', { name: 'Motor 1 Click to spin' })).toBeEnabled();
});

test('switching quickly in and out of the PX4 radio page raises no duplicate-command error', async ({ vehicle }) => {
  await openAircraftSetup(vehicle);
  const entry = (name: string) => vehicle.getByRole('button', { name: new RegExp(`^${name}( |$)`) }).first();
  await ['Radio', 'Safety', 'Radio', 'Safety', 'Radio', 'Safety'].reduce((done, page) => done.then(() => entry(page).click()), Promise.resolve());
  await expect(vehicle.getByText('Safety', { exact: true }).nth(1)).toBeVisible();
  await expect(vehicle.getByRole('dialog').filter({ hasText: 'Unable to send command' })).toHaveCount(0);
});

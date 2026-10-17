import { connectMock, expect, openAircraftSetup, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('ArduPilot setup names the frame and failsafe actions from the parameter metadata', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await expect(app.getByRole('button', { name: /^Frame Quad/ })).toBeVisible();
  await expect(app.getByRole('button', { name: /^Failsafes .*RTL/ })).toBeVisible();
  await expect(app.getByRole('button', { name: /^Flight modes .*Stabilize/ })).toBeVisible();
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
  await expect(vehicle.getByRole('button', { name: 'Motor 1 Tap to spin' })).toBeEnabled();
  const throttle = vehicle.getByRole('slider', { name: 'Throttle' });
  await throttle.press('ArrowRight');
  await throttle.press('ArrowRight');
  await throttle.press('ArrowRight');
  await expect(throttle).toHaveAttribute('aria-valuenow', '3');
  await expect(vehicle.getByText('3%', { exact: true })).toBeVisible();
  await vehicle.getByRole('button', { name: 'Motor 1 Tap to spin' }).click();
  await vehicle.getByRole('button', { name: 'Stop' }).click();
  await expect(vehicle.getByRole('button', { name: 'Motor 1 Tap to spin' })).toBeEnabled();
});

import { expect, flightMode, slideAcross, takeOff, telemetry, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('takeoff climbs to the chosen height and holds there', async ({ vehicle }) => {
  await takeOff(vehicle, '10 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^(Takeoff|Hold) Flying/);
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await expect(telemetry(vehicle, 'H')).toHaveText(/^H(9\.[5-9]|10\.[0-4])m$/);
});

test('slide to land brings the aircraft down and offers takeoff again', async ({ vehicle }) => {
  await takeOff(vehicle, '5 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to land' }));
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Land/);
  await expect(vehicle.getByRole('button', { name: 'Take off' })).toBeVisible({ timeout: 90_000 });
});

test('return switches the aircraft into return mode', async ({ vehicle }) => {
  await takeOff(vehicle, '5 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await vehicle.getByRole('button', { name: 'Return', exact: true }).click();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to return' }));
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Return/);
});

test('the flight mode menu changes the mode', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'Change flight mode', exact: true }).click();
  const menu = vehicle.getByRole('menu');
  await expect(menu).toBeVisible();
  await menu.getByRole('menuitem', { name: 'Position' }).click();
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Position/);
});

test('Escape cancels the takeoff panel', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'Take off' }).click();
  await expect(vehicle.getByText('Height above launch')).toBeVisible();
  await vehicle.keyboard.press('Escape');
  await expect(vehicle.getByText('Height above launch')).toBeHidden();
});

import { connectMock, expect, flightMode, mapMenuAt, showMapFull, slideAcross, takeOff, telemetry, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('on the ground the map menu offers a new home position', async ({ vehicle }) => {
  await showMapFull(vehicle);
  await mapMenuAt(vehicle, 200, -100);
  await expect(vehicle.getByText('Right-click the map to fly there')).toBeVisible();
  await expect(vehicle.getByRole('button', { name: 'Set home here' })).toBeVisible();
  await expect(vehicle.getByText(/^Lat: -?\d+\.\d{6}$/)).toBeVisible();
});

test('in flight the map menu flies to the clicked point', async ({ vehicle }) => {
  await takeOff(vehicle, '10 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await showMapFull(vehicle);
  await mapMenuAt(vehicle, 200, -100);
  await vehicle.getByRole('button', { name: 'Go to location' }).click();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to go to location' }));
  await expect(vehicle.getByText('Going to point')).toBeVisible();
  await expect(vehicle.getByText(/^Arrived/)).toBeVisible({ timeout: 90_000 });
  await expect(vehicle.getByText(/transfer failed/)).toBeHidden();
});

test('in flight an ArduCopter flies to the clicked point without saying the transfer failed', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await takeOff(vehicle, '10 m');
  await expect(telemetry(vehicle, 'H')).toHaveText(/^H(9\.[5-9]|10\.[0-4])m$/, { timeout: 60_000 });
  await showMapFull(vehicle);
  await mapMenuAt(vehicle, 200, -100);
  await vehicle.getByRole('button', { name: 'Go to location' }).click();
  const slide = vehicle.getByRole('button', { name: 'Slide to go to location' });
  await expect(async () => {
    if (await slide.isVisible()) await slideAcross(vehicle, slide);
    await expect(slide).toBeHidden({ timeout: 2_000 });
  }).toPass({ timeout: 20_000 });
  await expect(vehicle.getByText(/^Arrived/)).toBeVisible({ timeout: 90_000 });
  await expect(vehicle.getByText(/transfer failed/)).toBeHidden();
});

test('in flight the map menu offers an orbit with a radius range', async ({ vehicle }) => {
  await takeOff(vehicle, '10 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await showMapFull(vehicle);
  await mapMenuAt(vehicle, 150, -80);
  await vehicle.getByRole('button', { name: 'Orbit at location' }).click();
  await expect(vehicle.getByText('Orbit the vehicle around the specified location')).toBeVisible();
  await expect(vehicle.getByText(/^[\d.]+ to [\d.]+ m$/)).toBeVisible();
  await expect(vehicle.getByRole('button', { name: 'Slide to orbit' })).toBeVisible();
  await vehicle.getByRole('button', { name: 'Cancel' }).click();
  await expect(vehicle.getByRole('button', { name: 'Slide to orbit' })).toBeHidden();
});

test('in flight the map menu flies an orbit around the clicked point', async ({ vehicle }) => {
  await takeOff(vehicle, '10 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await showMapFull(vehicle);
  await mapMenuAt(vehicle, 150, -80);
  await vehicle.getByRole('button', { name: 'Orbit at location' }).click();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to orbit' }));
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Orbit/, { timeout: 30_000 });
});

import { connectMock, expect, openMenu, test } from './fixtures';

const PAGES = ['Flight logs', 'Geotag images', 'Vibration', 'MAVLink inspector', 'Console', 'Messages', 'Firmware'] as const;

PAGES.map((page) =>
  test(`Analyze › ${page} opens with the vehicle connected`, async ({ vehicle }) => {
    await openMenu(vehicle, 'Analyze');
    const row = vehicle.getByRole('button', { name: new RegExp(`^${page}`) });
    const titled = vehicle.getByText(page, { exact: true });
    const before = await titled.count();
    await row.click();
    await expect.poll(() => titled.count()).toBeGreaterThan(before);
  }),
);

test('the MAVLink inspector lists the vehicle heartbeat', async ({ vehicle }) => {
  await openMenu(vehicle, 'Analyze');
  await vehicle.getByRole('button', { name: /^MAVLink inspector/ }).click();
  await expect(vehicle.getByText(/HEARTBEAT/).first()).toBeVisible();
});

test('the console sends a shell command and shows the vehicle answer', async ({ vehicle }) => {
  await openMenu(vehicle, 'Analyze');
  await vehicle.getByRole('button', { name: /^Console/ }).click();
  const command = vehicle.getByRole('textbox', { name: 'Enter commands here...' });
  await command.fill('ver all');
  await vehicle.getByRole('button', { name: 'Send' }).click();
  await expect(vehicle.getByText(/HW arch: MOCK_SITL/)).toBeVisible({ timeout: 15_000 });
  await command.fill('frobnicate');
  await command.press('Enter');
  await expect(vehicle.getByText(/nsh: frobnicate: command not found/)).toBeVisible({ timeout: 15_000 });
});

test('vibration shows live levels on all three axes', async ({ vehicle }) => {
  await openMenu(vehicle, 'Analyze');
  const vibration = vehicle.getByRole('button', { name: /^Vibration X \d+ · Y \d+ · Z \d+ m\/s²/ });
  await expect(vibration).toBeVisible({ timeout: 30_000 });
  await vibration.click();
  await expect(vehicle.getByText('Vibration (m/s²)')).toBeVisible();
});

test('messages lists what the vehicle has said', async ({ app }) => {
  await connectMock(app, ['Send status text + voice']);
  await openMenu(app, 'Analyze');
  await app.getByRole('button', { name: /^Messages/ }).click();
  await expect(app.getByText('Status text warning').last()).toBeVisible({ timeout: 30_000 });
  await expect(app.getByText('No messages')).toBeHidden();
});

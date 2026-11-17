import { closeSettings, connectMock, expect, flightMode, openMenu, takeOff, telemetry, test } from './fixtures';
import type { Page } from '@playwright/test';

const openGeneral = async (page: Page) => {
  await openMenu(page, 'Settings');
  await page.getByRole('tablist').getByRole('tab', { name: 'General' }).click();
};

test('theme switches between dark and light and survives a reload', async ({ app }) => {
  await openGeneral(app);
  await expect(app.locator('html')).toHaveClass(/dark/);
  await app.getByRole('radio', { name: 'Light' }).click();
  await expect(app.locator('html')).toHaveClass(/light/);
  await app.reload();
  await expect(app.locator('html')).toHaveClass(/light/);
});

test('a switch setting keeps its value across a reload', async ({ app }) => {
  await openGeneral(app);
  const mute = app.getByRole('switch', { name: 'Mute all audio output' });
  await expect(mute).not.toBeChecked();
  await mute.click();
  await expect(mute).toBeChecked();
  await app.reload();
  await openGeneral(app);
  await expect(app.getByRole('switch', { name: 'Mute all audio output' })).toBeChecked();
});

test('the enforce checklist switch unlocks only with the checklist on', async ({ app }) => {
  await openGeneral(app);
  const enforce = app.getByRole('switch', { name: 'Enforce preflight checklist' });
  await expect(enforce).toBeDisabled();
  await app.getByRole('switch', { name: 'Use preflight checklist' }).click();
  await expect(enforce).toBeEnabled();
});

test('on a desktop a number is typed in its row and written only on Enter', async ({ app }) => {
  await openGeneral(app);
  const altitude = app.getByRole('textbox', { name: 'Default waypoint altitude' });
  await expect(altitude).toHaveValue('50');
  await altitude.fill('63');
  await app.keyboard.press('Escape');
  await expect(altitude).toHaveValue('50');
  await altitude.fill('64');
  await app.getByRole('tablist').getByRole('tab', { name: 'General' }).focus();
  await app.reload();
  await openGeneral(app);
  await expect(altitude).toHaveValue('50');
  await altitude.fill('65');
  await altitude.press('Enter');
  await expect(app.getByText('Default waypoint altitude: 65 m')).toBeVisible();
  await app.reload();
  await openGeneral(app);
  await expect(altitude).toHaveValue('65');
});

test('the settings tabs move with the arrow keys and the find shortcut jumps to search', async ({ app }) => {
  await openGeneral(app);
  const tabs = app.getByRole('tablist');
  await tabs.getByRole('tab', { name: 'General' }).focus();
  await app.keyboard.press('ArrowDown');
  await expect(tabs.getByRole('tab', { name: 'Safety' })).toBeFocused();
  await expect(tabs.getByRole('tab', { name: 'Safety' })).toHaveAttribute('aria-selected', 'true');
  await app.keyboard.press('End');
  await expect(tabs.getByRole('tab', { name: 'Aircast device' })).toHaveAttribute('aria-selected', 'true');
  await expect(tabs.locator('[tabindex="0"]')).toHaveCount(1);
  await app.keyboard.press('ControlOrMeta+f');
  await expect(app.getByPlaceholder('Search settings')).toBeFocused();
});

test('settings search finds a setting by name', async ({ app }) => {
  await openMenu(app, 'Settings');
  await app.getByPlaceholder('Search settings').fill('mute');
  await expect(app.getByText('Mute all audio output')).toBeVisible();
});

test('settings search finds the payload bays by what operators call a drop', async ({ app }) => {
  await openMenu(app, 'Settings');
  const search = app.getByPlaceholder('Search settings');
  await ['gripper', 'drop', 'release'].reduce(async (previous, word) => {
    await previous;
    await search.fill(word);
    await expect(app.getByText('Payload bays', { exact: true })).toBeVisible();
  }, Promise.resolve());
});

test('turning on the ADS-B server connection keeps the core running', async ({ app }) => {
  await openMenu(app, 'Settings');
  await app.getByPlaceholder('Search settings').fill('ADS-B');
  const adsb = app.getByRole('switch', { name: 'Show nearby aircraft (ADS-B)' });
  await adsb.click();
  await expect(adsb).toBeChecked();
  await closeSettings(app);
  await connectMock(app);
});

test('turning on the on-screen sticks shows both sticks on the Fly screen', async ({ vehicle }) => {
  await openMenu(vehicle, 'Settings');
  await vehicle.getByRole('tablist').getByRole('tab', { name: 'General' }).click();
  const sticks = vehicle.getByRole('switch', { name: 'Enabled' });
  await sticks.click();
  await expect(sticks).toBeChecked();
  await expect(vehicle.getByRole('switch', { name: 'Left-handed mode (swap sticks)' })).toBeEnabled();
  await closeSettings(vehicle);
  await expect(vehicle.getByLabel('Left virtual stick')).toBeVisible({ timeout: 60_000 });
  await expect(vehicle.getByLabel('Right virtual stick')).toBeVisible();
});

test('imperial units turn the Fly telemetry into feet', async ({ vehicle }) => {
  await expect(telemetry(vehicle, 'H')).toContainText('m');
  await openGeneral(vehicle);
  await vehicle.getByRole('button', { name: 'Imperial', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: 'Imperial', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await closeSettings(vehicle);
  await expect(telemetry(vehicle, 'H')).toContainText('ft');
});

test('the radio on USB settings sit with the other stick inputs and let the browser grant the radio its serial port', async ({ app }) => {
  await openMenu(app, 'Settings');
  await app.getByRole('tablist').getByRole('tab', { name: 'Control' }).click();
  await expect(app.getByText('Radio on USB (EdgeTX script)', { exact: false })).toBeVisible();
  await expect(app.getByRole('button', { name: 'Allow a USB port' })).toBeVisible();
});

test('a limit the disabled geofence does not enforce reads Off until it is turned on, and Undo turns it back off', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openMenu(app, 'Settings');
  await app.getByRole('tablist').getByRole('tab', { name: 'Safety' }).click();
  const fenceOff = app.getByText('The geofence is off, so nothing holds the aircraft to this.');
  await expect(fenceOff).toHaveCount(2, { timeout: 60_000 });
  await expect(app.getByRole('slider', { name: 'Max altitude' })).toHaveCount(0);
  await app.getByRole('button', { name: 'Turn on' }).first().click();
  await expect(app.getByRole('slider', { name: 'Max altitude' })).toBeVisible();
  await expect(fenceOff).toHaveCount(1);
  await app.getByRole('status').getByRole('button', { name: 'Undo' }).click();
  await expect(fenceOff).toHaveCount(2);
});

test('turning a geofence limit on while armed asks first, since the breach action may start at once', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await takeOff(app, '5 m');
  await expect(flightMode(app)).toHaveAccessibleName(/Flying/, { timeout: 60_000 });
  await openMenu(app, 'Settings');
  await app.getByRole('tablist').getByRole('tab', { name: 'Safety' }).click();
  const fenceOff = app.getByText('The geofence is off, so nothing holds the aircraft to this.');
  await expect(fenceOff).toHaveCount(2, { timeout: 60_000 });
  await app.getByRole('button', { name: 'Turn on' }).first().click();
  const confirm = app.getByRole('alertdialog').or(app.getByRole('dialog')).filter({ hasText: 'Turn on max altitude while armed?' });
  await expect(confirm).toContainText('breach action');
  await confirm.getByRole('button', { name: 'Cancel' }).click();
  await expect(confirm).toBeHidden();
  await expect(fenceOff).toHaveCount(2);
  await app.getByRole('button', { name: 'Turn on' }).first().click();
  await confirm.getByRole('button', { name: 'Turn on' }).click();
  await expect(app.getByRole('slider', { name: 'Max altitude' })).toBeVisible();
  await expect(fenceOff, 'only the chosen limit comes on').toHaveCount(1);
});

test('settings search reaches setup pages by what they hold, and the parameters', async ({ vehicle }) => {
  await openMenu(vehicle, 'Settings');
  const search = vehicle.getByPlaceholder('Search settings');
  await search.fill('compass');
  await expect(vehicle.getByRole('button', { name: /^Sensors/ })).toBeVisible();
  await search.fill('calibrat');
  await expect(vehicle.getByRole('button', { name: /^Sensors/ })).toBeVisible();
  await expect(vehicle.getByRole('button', { name: /^Radio/ })).toBeVisible();
  await search.fill('parameters');
  await vehicle.getByRole('button', { name: /^Parameters Every setting/ }).click();
  await expect(vehicle.getByRole('textbox', { name: 'Search parameters' })).toBeVisible();
});

test('the two camera channel groups in Control say which radio they read', async ({ app }) => {
  await openMenu(app, 'Settings');
  await app.getByRole('tablist').getByRole('tab', { name: 'Control' }).click();
  await expect(app.getByText("Camera & gimbal on the aircraft's receiver", { exact: true })).toBeVisible();
  await expect(app.getByText('Radio on USB (EdgeTX script)', { exact: false })).toBeVisible();
});

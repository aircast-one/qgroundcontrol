import { expect, openMenu, test } from './fixtures';

const PAGES: readonly (readonly [string, string, string?])[] = [
  ['Camera', 'Video sources'],
  ['General', 'Maps'],
  ['Transmission', 'MAVLink'],
  ['Transmission', 'Packet radio'],
  ['Transmission', 'RTK GPS'],
  ['Transmission', 'NTRIP / RTK'],
  ['Safety', 'Remote ID'],
  ['Safety', 'More failsafe settings', 'Aircraft setup'],
  ['General', 'PX4 log transfer'],
  ['General', 'App logging'],
  ['General', 'App log$', 'App log'],
  ['General', 'About'],
  ['General', 'Aircast card'],
];

PAGES.map(([tab, page, title = page]) =>
  test(`${tab} › ${page} opens and goes back`, async ({ vehicle }) => {
    await openMenu(vehicle, 'Settings');
    await vehicle.getByRole('tablist').getByRole('tab', { name: tab }).click();
    const row = vehicle.getByRole('button', { name: new RegExp(`^${page.replace(/[/]/g, '\\/')}`) });
    await expect(async () => {
      if (await row.isVisible()) await row.click();
      await expect(vehicle.getByRole('button', { name: 'Back', exact: true })).toBeVisible({ timeout: 2_000 });
    }).toPass({ timeout: 30_000 });
    await expect(vehicle.getByText(title, { exact: true }).first()).toBeVisible();
    await vehicle.getByRole('button', { name: 'Back', exact: true }).click();
    await expect(row).toBeVisible();
  }),
);

test('a setup row clicked before the tab pager settles opens its page and stays there', async ({ vehicle }) => {
  await openMenu(vehicle, 'Settings');
  await expect(vehicle.getByRole('button', { name: /^Aircast card/ })).toBeVisible();
  await vehicle.clock.install();
  await vehicle.clock.pauseAt(await vehicle.evaluate(() => Date.now() + 1_000));
  await vehicle.getByRole('tablist').getByRole('tab', { name: 'Safety' }).click();
  await vehicle.getByRole('button', { name: /^More failsafe settings/ }).click();
  await expect(vehicle.getByRole('button', { name: 'Back', exact: true })).toBeVisible();
  await vehicle.clock.runFor(1_000);
  await expect(vehicle.getByRole('button', { name: 'Back', exact: true })).toBeVisible();
});

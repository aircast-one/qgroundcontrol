import { expect, test } from './fixtures';

const PAGES: readonly (readonly [string, string, string?])[] = [
  ['Transmission', 'Video sources'],
  ['Transmission', 'MAVLink'],
  ['Transmission', 'Packet radio'],
  ['Transmission', 'RTK GPS'],
  ['Transmission', 'NTRIP / RTK'],
  ['Safety', 'Remote ID'],
  ['Safety', 'More failsafe settings', 'Aircraft setup'],
];

PAGES.map(([tab, page, title = page]) =>
  test(`${tab} › ${page} opens and goes back`, async ({ vehicle }) => {
    await vehicle.getByRole('button', { name: 'Settings', exact: true }).click();
    await vehicle.getByRole('tablist').getByRole('tab', { name: tab }).click();
    const row = vehicle.getByRole('button', { name: new RegExp(`^${page.replace(/[/]/g, '\\/')}`) });
    await expect(async () => {
      await row.click();
      await expect(vehicle.getByRole('button', { name: 'Back', exact: true })).toBeVisible({ timeout: 1_000 });
    }).toPass({ timeout: 15_000 });
    await expect(vehicle.getByText(title, { exact: true }).first()).toBeVisible();
    await vehicle.getByRole('button', { name: 'Back', exact: true }).click();
    await expect(row).toBeVisible();
  }),
);

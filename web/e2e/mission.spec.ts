import type { Page } from '@playwright/test';
import { connectMock, expect, flightMode, openPlanAtVehicle, slideAcross, tapPlanMission, telemetry, test } from './fixtures';

test.describe.configure({ timeout: 180_000 });

test('an uploaded plan flies as a mission with a progress card', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await vehicle.getByRole('button', { name: 'Upload', exact: true }).click();
  const fly = vehicle.getByRole('button', { name: 'Fly', exact: true }).last();
  await expect(vehicle.getByRole('button', { name: 'Upload', exact: true })).toBeHidden({ timeout: 30_000 });
  await fly.click();
  await expect(vehicle.getByText('Start mission', { exact: true })).toBeVisible();
  await expect(vehicle.getByText(/^Not fully ready\./), 'starting a mission takes off, so it warns like Take off does').toBeVisible();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to start mission anyway', exact: true }));
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Mission/, { timeout: 30_000 });
  await expect(vehicle.getByText(/^To waypoint \d+ of \d+/)).toBeVisible({ timeout: 30_000 });
  await expect(vehicle.getByRole('button', { name: 'Skip' })).toBeVisible();
});

test('skip moves a running mission on to the next waypoint', async ({ vehicle }) => {
  await openPlanAtVehicle(vehicle);
  await tapPlanMission(vehicle);
  await vehicle.getByRole('button', { name: 'Upload', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: 'Upload', exact: true })).toBeHidden({ timeout: 30_000 });
  await vehicle.getByRole('button', { name: 'Fly', exact: true }).last().click();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to start mission' }));
  const progress = vehicle.getByText(/^To waypoint \d+ of \d+/);
  await expect(progress).toBeVisible({ timeout: 30_000 });
  const [, before] = (await progress.textContent())?.match(/^To waypoint (\d+)/) ?? [];
  await vehicle.getByRole('button', { name: 'Skip' }).click();
  await expect(vehicle.getByText(/^Adjust current waypoint to \d+$/)).toBeVisible();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to set waypoint' }));
  await expect.poll(async () => Number((await progress.textContent())?.match(/^To waypoint (\d+)/)?.[1]), { timeout: 30_000 }).toBeGreaterThan(Number(before));
});

const flyUploadedMission = async (page: Page) => {
  await openPlanAtVehicle(page);
  await tapPlanMission(page);
  await page.getByRole('button', { name: 'Upload', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Upload', exact: true })).toBeHidden({ timeout: 30_000 });
  await page.getByRole('button', { name: 'Fly', exact: true }).last().click();
  await slideAcross(page, page.getByRole('button', { name: 'Slide to start mission' }));
  await expect(page.getByText(/^To waypoint \d+ of \d+/)).toBeVisible({ timeout: 60_000 });
};

const leftOverAction = (page: Page) => page.getByRole('dialog').filter({ hasText: 'Leave plan on vehicle' });

test('landing out of a flown mission says where it stopped, and removing the plan is not the first choice', async ({ vehicle }) => {
  await flyUploadedMission(vehicle);
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to land' }));
  const done = leftOverAction(vehicle);
  await expect(done).toBeVisible({ timeout: 120_000 });
  await expect(done.getByText(/^Mission stopped at waypoint \d+ of \d+$/), 'a mission left early is not complete').toBeVisible();
  const choices = await done.getByRole('button').allTextContents();
  expect(choices.indexOf('Remove plan from vehicle'), 'removing the plan comes after keeping it').toBeGreaterThan(choices.indexOf('Leave plan on vehicle'));
  await done.getByRole('button', { name: 'Close' }).click();
  await expect(done).toBeHidden();
});

test('an ArduCopter mission left with Return names its waypoint and offers to resume', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await flyUploadedMission(app);
  await expect(app.getByText(/^To waypoint [3-9] of \d+/)).toBeVisible({ timeout: 120_000 });
  await app.getByRole('button', { name: 'Return', exact: true }).click();
  await slideAcross(app, app.getByRole('button', { name: 'Slide to return' }));
  const done = leftOverAction(app);
  await expect(done).toBeVisible({ timeout: 180_000 });
  await expect(done.getByText(/^Mission stopped at waypoint [3-9] of \d+$/)).toBeVisible();
  await expect(done.getByRole('button', { name: /^Resume from the waypoint \(\d+ of \d+\)$/ }), 'the title names a plan row, so the button names one too rather than the vehicle command number').toBeVisible();
});

test('the mission card over the picture leaves the home readings clear and names a distance only once the vehicle reports one', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await flyUploadedMission(app);
  const line = app.getByText(/^To waypoint \d+ of \d+/);
  await expect(line, 'ArduCopter streams NAV_CONTROLLER_OUTPUT, so the distance is real').toHaveText(/^To waypoint \d+ of \d+ · [1-9]\d* m$/, { timeout: 30_000 });
  const card = (await line.locator('xpath=../../..').boundingBox())!;
  const readings = await Promise.all(['D', 'H'].map(async (label) => (await telemetry(app, label).boundingBox())!));
  expect(readings.filter((it) => it.x < card.x + card.width && card.x < it.x + it.width && it.y < card.y + card.height && card.y < it.y + it.height), 'the card sits above the home readings, not on them').toEqual([]);
});

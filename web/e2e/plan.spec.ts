import { expect, openMenu, test } from './fixtures';
import type { Page } from '@playwright/test';

test.describe.configure({ timeout: 120_000 });

const itemCount = async (page: Page): Promise<number> => {
  const listed = page.getByText(/^\d+ items$/);
  return (await listed.count()) === 0 ? 0 : Number.parseInt((await listed.first().textContent()) ?? '0', 10);
};

const tapMission = async (page: Page) => {
  const box = await page.getByRole('region', { name: 'Map' }).boundingBox();
  if (box === null) throw new Error('plan map is not on screen');
  const [x, y] = [box.x + box.width * 0.6, box.y + box.height * 0.5];
  await [[0, 0], [80, -60], [160, 0], [80, 80]].reduce(
    (done, [dx, dy]) => done.then(async () => {
      const before = await itemCount(page);
      await page.mouse.click(x + dx!, y + dy!);
      await expect.poll(() => itemCount(page)).toBeGreaterThan(before);
    }),
    Promise.resolve(),
  );
};

test('tapping the plan map builds a mission with home, takeoff and waypoints', async ({ app }) => {
  await openMenu(app, 'Plan');
  await expect(app.getByText('Tap the map to add a waypoint')).toBeVisible();
  await tapMission(app);
  await expect(app.getByText(/^5 items · \d+ m/)).toBeVisible();
  await app.getByRole('button', { name: 'Undo' }).click();
  await expect(app.getByText(/^(Empty plan|[1-4] items · .*)$/)).toBeVisible();
});

test('upload sends the mission to the vehicle and offers to fly it', async ({ vehicle }) => {
  await vehicle.addLocatorHandler(vehicle.getByRole('dialog').filter({ hasText: 'Aircraft connected' }), (prompt) => prompt.getByRole('button', { name: 'Keep my route' }).click());
  await openMenu(vehicle, 'Plan');
  await expect(vehicle.getByText('Tap the map to add a waypoint')).toBeVisible();
  await expect(vehicle.locator('span.text-label-small').getByText(/^\d+ (m|km)$/)).toHaveText(/^\d+ m$/, { timeout: 60_000 });
  await tapMission(vehicle);
  await vehicle.getByRole('button', { name: 'Upload', exact: true }).click();
  await expect(vehicle.getByRole('button', { name: 'Upload', exact: true })).toBeHidden({ timeout: 30_000 });
  await expect(vehicle.getByRole('button', { name: 'Fly', exact: true }).last()).toBeVisible();
});

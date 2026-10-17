import { APP, expect, test } from './fixtures';

test('a second tab waits instead of running a second copy, and Use here moves Aircast to it', async ({ app, context }) => {
  const second = await context.newPage();
  await second.goto(APP);
  await expect(second.getByRole('heading', { name: 'Aircast is open in another tab' })).toBeVisible();
  await second.getByRole('button', { name: 'Use here' }).click();
  await expect(app.getByRole('heading', { name: 'Aircast is open in another tab' })).toBeVisible({ timeout: 30_000 });
  await expect(second.getByRole('button', { name: /Not connected|How to connect/ }).first()).toBeVisible({ timeout: 60_000 });
});

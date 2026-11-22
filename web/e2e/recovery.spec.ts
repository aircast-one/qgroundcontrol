import type { Page } from '@playwright/test';
import { expect, flightMode, test } from './fixtures';

const DEBUG_UI = 'one.aircast.android.DEBUG_UI';
const FAILURE = 'Error: The debug UI asked this screen to fail';

const failDrawing = (page: Page) => page.evaluate((action) => (window as unknown as Record<string, (cmd: string) => string>)[action]?.('crash'), DEBUG_UI);

const pageErrors = (page: Page) =>
  page.evaluate(async (module) => {
    const { Qgc } = (await import(module)) as { Qgc: { get: (path: string) => Promise<{ entries?: { message: string }[] } | null> } };
    return ((await Qgc.get('view.appLog(3,page,,0,)'))?.entries ?? []).map((it) => it.message);
  }, '/src/bridge/Qgc.ts');

test('a screen that fails while drawing comes back, and failing again shows Reload and Copy details instead of a blank window', async ({ vehicle }) => {
  await vehicle.context().grantPermissions(['clipboard-read', 'clipboard-write']);
  await failDrawing(vehicle);
  await expect(flightMode(vehicle), 'the app drew itself again').toBeVisible();
  await expect.poll(() => pageErrors(vehicle)).toEqual([expect.stringMatching(new RegExp(`^C event=page_error kind=render reason="${FAILURE}" stack=".+" components=".*AircastShell.*" screen=\\{"tab":"Fly".* head=browser version=\\S+ sha=\\S+ uptime_s=\\d+\\.\\d$`))]);

  await failDrawing(vehicle);
  await expect(vehicle.getByText('This screen stopped working')).toBeVisible();
  await expect(vehicle.getByText(FAILURE, { exact: true })).toBeVisible();
  await vehicle.getByRole('button', { name: 'Copy details' }).click();
  await expect(vehicle.getByRole('button', { name: 'Copied' })).toBeVisible();
  expect(await vehicle.evaluate(() => navigator.clipboard.readText())).toBe((await pageErrors(vehicle)).at(-1)?.replace(/^C /, ''));

  await vehicle.getByRole('button', { name: 'Reload' }).click();
  await expect(vehicle.getByRole('region', { name: 'Map' })).toBeVisible();
});

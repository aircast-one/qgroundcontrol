import type { Page } from '@playwright/test';
import { connectMock, expect, mapMenuAt, showMapFull, slideAcross, takeOff, telemetry, test } from './fixtures';

test.describe.configure({ timeout: 240_000 });

const PROOF = process.env.INTERCEPT_PROOF;

const offeredConfirm = (page: Page) =>
  page.evaluate(
    async (module) => ((await import(module)) as { Qgc: { path: (path: string) => { entry?: { confirm?: { text?: string } | null } } | null } }).Qgc.path('view.autoIntercept')?.entry?.confirm?.text ?? '',
    '/src/bridge/Qgc.ts',
  );

test('auto intercept on a marked point asks with the core words, then the pill follows the run', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await takeOff(vehicle, '10 m');
  await expect(telemetry(vehicle, 'H')).toHaveText(/^H(9\.[5-9]|10\.[0-4])m$/, { timeout: 60_000 });
  await showMapFull(vehicle);
  await mapMenuAt(vehicle, 120, -80);
  await vehicle.getByRole('button', { name: 'Mark as target' }).click();
  await expect.poll(() => offeredConfirm(vehicle)).toMatch(/^Chase the marked point/);
  const asked = vehicle.getByText(/^Chase the marked point/);
  await expect(async () => {
    await vehicle.getByRole('button', { name: 'More' }).click();
    await vehicle.getByRole('button', { name: 'Auto Intercept' }).click();
    await expect(asked)
      .toBeVisible({ timeout: 1_000 })
      .catch(async (missing: unknown) => {
        await vehicle.getByRole('button', { name: 'Cancel' }).click();
        throw missing;
      });
  }).toPass({ timeout: 20_000 });
  if (PROOF) await vehicle.screenshot({ path: `${PROOF}/web-confirm.png` });
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to auto intercept' }));
  await expect(vehicle.getByText(/^(Auto Intercept starting|Chasing the marked point)/)).toBeVisible({ timeout: 20_000 });
  await expect(vehicle.getByRole('button', { name: 'Stop', exact: true })).toBeVisible();
  if (PROOF) await vehicle.screenshot({ path: `${PROOF}/web-pill.png` });
  const outcome = vehicle.getByText(/^(Hit the|Missed by|Dropped|Couldn't line up|Auto Intercept stopped)/);
  await expect(outcome).toBeVisible({ timeout: 180_000 });
  if (PROOF) await vehicle.screenshot({ path: `${PROOF}/web-outcome.png` });
});

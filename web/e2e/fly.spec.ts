import type { Page } from '@playwright/test';
import { expect, flightMode, slideAcross, takeOff, telemetry, test } from './fixtures';
import { type Sitl, connectSitl, startSitl } from './sitl';

test.describe.configure({ timeout: 120_000 });

test('takeoff climbs to the chosen height and holds there', async ({ vehicle }) => {
  await takeOff(vehicle, '10 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^(Takeoff|Hold) Flying/);
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await expect(telemetry(vehicle, 'H')).toHaveText(/^H(9\.[5-9]|10\.[0-4])m$/);
});

test('slide to land brings the aircraft down and offers takeoff again', async ({ vehicle }) => {
  await takeOff(vehicle, '5 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to land' }));
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Land/);
  await expect(vehicle.getByRole('button', { name: 'Take off' })).toBeVisible({ timeout: 90_000 });
});

test('return switches the aircraft into return mode', async ({ vehicle }) => {
  await takeOff(vehicle, '5 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await vehicle.getByRole('button', { name: 'Return', exact: true }).click();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to return' }));
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Return/);
});

test('the flight mode menu changes the mode', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'Change flight mode', exact: true }).click();
  const menu = vehicle.getByRole('menu');
  await expect(menu).toBeVisible();
  await menu.getByRole('menuitem', { name: 'Position' }).click();
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Position/);
});

test('Escape cancels the takeoff panel', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'Take off' }).click();
  await expect(vehicle.getByText('Height above launch')).toBeVisible();
  await vehicle.keyboard.press('Escape');
  await expect(vehicle.getByText('Height above launch')).toBeHidden();
});

const ARM_VOLTAGE = 'vehicle.parameterManager.getParameter(-1,BATT_ARM_VOLT)';

const core = (page: Page, path: string, value?: unknown): Promise<{ valid?: boolean; value?: unknown }> =>
  page.evaluate(
    async ({ module, path, value }) => {
      const { Qgc } = (await import(module)) as { Qgc: { get: (path: string) => Promise<unknown>; set: (path: string, value: unknown) => Promise<unknown> } };
      return (await (value === undefined ? Qgc.get(path) : Qgc.set(path, value))) as { valid?: boolean; value?: unknown };
    },
    { module: '/src/bridge/Qgc.ts', path, value },
  );

test.describe('against ArduPilot', () => {
  const sitl: { running: Sitl | null } = { running: null };

  test.describe.configure({ mode: 'serial' });

  test.beforeAll(async () => {
    test.setTimeout(120_000);
    const running = startSitl();
    sitl.running = running;
    await expect.poll(() => running.heard.heartbeat, { timeout: 90_000 }).toBe(true);
  });

  test.afterAll(() => sitl.running?.stop());

  test('a takeoff the aircraft refuses to arm for says why, once', async ({ app }) => {
    test.setTimeout(300_000);
    await app.addLocatorHandler(app.getByText('Configuration tasks remain before this vehicle is ready to fly.', { exact: false }), () => app.getByRole('button', { name: 'Later', exact: true }).click());
    await connectSitl(app, sitl.running?.ws ?? '');
    await expect.poll(async () => (await core(app, 'vehicle.homePosition')).valid, { timeout: 180_000 }).toBe(true);
    await expect
      .poll(
        async () => {
          await core(app, ARM_VOLTAGE, 30);
          return (await core(app, ARM_VOLTAGE)).value;
        },
        { timeout: 60_000 },
      )
      .toBe(30);
    await app.getByRole('button', { name: 'Take off', exact: true }).click();
    await app.getByRole('button', { name: '5 m', exact: true }).click();
    await slideAcross(app, app.getByRole('button', { name: /^Slide to take off( anyway)? · / }));
    await expect(app.getByText("Can't arm: battery 1 below minimum arming voltage.")).toBeVisible({ timeout: 30_000 });
    await expect(app.getByText(/^(Can't arm|The aircraft refused to arm|The aircraft didn't answer|The aircraft accepted the arm)/)).toHaveCount(1);
    await expect(flightMode(app)).not.toHaveAccessibleName(/Flying/);
  });
});

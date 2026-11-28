import { test as base, expect, type Locator, type Page } from '@playwright/test';
import { DESKTOP_QUERY } from '../src/ui/DesktopLayout';

export const APP = '/';

const passFirstRun = async (page: Page) => {
  const welcome = page.getByRole('dialog').filter({ has: page.getByRole('heading', { name: 'Welcome to Aircast' }) });
  await welcome.getByRole('button', { name: 'Continue' }).click();
  await expect(welcome).toBeHidden();
};

export const openApp = async (page: Page, { firstRun = true } = {}) => {
  await page.goto(APP);
  if (!firstRun) return;
  await passFirstRun(page);
  await expect(page.getByRole('region', { name: 'Map' })).toBeVisible();
};

export const openMenu = async (page: Page, item: string) => {
  const rail = page.getByRole('navigation').getByRole('button', { name: item, exact: true });
  const menu = page.getByRole('button', { name: 'Menu', exact: true });
  await expect(rail.or(menu).first()).toBeVisible();
  if (await rail.isVisible()) return rail.click();
  await menu.click();
  await page.getByRole('dialog').getByRole('button', { name: item, exact: true }).click();
};

export const closeSettings = async (page: Page) => {
  const desktop = await page.evaluate((query) => window.matchMedia(query).matches, DESKTOP_QUERY);
  const close = desktop ? page.getByRole('navigation').getByRole('button', { name: 'Fly', exact: true }) : page.getByRole('button', { name: 'Close settings' });
  await close.click();
  await expect(desktop ? page.getByRole('navigation', { name: 'Settings' }) : close).toBeHidden();
};

export const FRESH_PARAMETERS = 'Start with fresh firmware parameters (setup required)';

export type MockOption = 'Enable camera' | 'Enable gimbal' | 'Enable proximity sensors' | 'Send status text + voice' | typeof FRESH_PARAMETERS;

export type MockVehicle = 'PX4 Vehicle' | 'PX4 Fixed Wing Vehicle' | 'APM ArduCopter Vehicle' | 'APM ArduPlane Vehicle' | 'APM ArduSub Vehicle' | 'APM ArduRover Vehicle' | 'Generic Vehicle';

export const chooseCameraJoystick = (page: Page) =>
  page.evaluate(async (module) => {
    const { Qgc } = (await import(module)) as { Qgc: { set: (path: string, value: unknown) => Promise<unknown> } };
    await Qgc.set('settings.gimbalControllerSettings.onScreenStick', true);
  }, '/src/bridge/Qgc.ts');

export const startMock = async (page: Page, options: readonly MockOption[] = [], vehicle: MockVehicle | null = null) => {
  await page.getByRole('button', { name: /How to connect/ }).click();
  await page.getByRole('button', { name: 'Add a link' }).click();
  await page.getByRole('button', { name: /^Simulated vehicle/ }).click();
  if (vehicle !== null) {
    await page.getByRole('button', { name: 'Vehicle type' }).click();
    await page.getByRole('menuitem', { name: vehicle }).click();
  }
  await options.reduce((done, name) => done.then(() => page.getByRole('checkbox', { name }).check()), Promise.resolve());
  await page.getByRole('button', { name: 'Start', exact: true }).click();
};

export const connectMock = async (page: Page, options: readonly MockOption[] = [], vehicle: MockVehicle | null = null) => {
  await startMock(page, options, vehicle);
  await expect(page.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });
};

export const connectFreshMock = async (page: Page, vehicle: MockVehicle) => {
  await startMock(page, [FRESH_PARAMETERS], vehicle);
  const prompt = page.getByRole('dialog').filter({ hasText: /needs? setup before this vehicle can fly\./ });
  await expect(prompt).toBeVisible({ timeout: 60_000 });
  return prompt;
};

export const WHEP_STREAM = 'http://localhost:18889/e2e/whep';

export const addVideoSource = async (page: Page, type: RegExp, address: string) => {
  await openMenu(page, 'Settings');
  await page.getByRole('tablist').getByRole('tab', { name: 'Camera' }).click();
  const row = page.getByRole('button', { name: /^Video sources/ });
  await expect(async () => {
    if (await row.isVisible()) await row.click();
    await expect(page.getByRole('button', { name: 'Add a video source' })).toBeVisible({ timeout: 2_000 });
  }).toPass({ timeout: 30_000 });
  await page.getByRole('button', { name: 'Add a video source' }).click();
  await page.getByRole('dialog').filter({ hasText: 'Add video source' }).getByRole('button', { name: type }).click();
  await page.getByRole('textbox').first().fill(address);
  await page.getByRole('button', { name: 'Save' }).click();
  await expect(page.getByText(address)).toBeVisible();
  await closeSettings(page);
};

export const flightMode = (page: Page): Locator => page.getByRole('button', { name: /Change flight mode$/ }).first();

export const telemetry = (page: Page, label: string): Locator => page.getByText(label, { exact: true }).locator('..').filter({ hasText: new RegExp(`^${label.replace('.', '\\.')}-?[\\d.]+ (m|m/s|ft|ft/s)$`) });

const nextFrames = (page: Page) => page.evaluate(() => new Promise<void>((done) => requestAnimationFrame(() => requestAnimationFrame(() => done()))));

const steadyBox = async (page: Page, target: Locator) => {
  const read = async () => JSON.stringify(await target.boundingBox());
  await expect
    .poll(async () => {
      const before = await read();
      await nextFrames(page);
      return before === (await read()) && before !== 'null';
    })
    .toBe(true);
  return target.boundingBox();
};

const MOUSE_POINTER = 1;

const ANYWAY = ' anyway';

const warnedMidSlide = (pressed: string | null, released: string | null): boolean =>
  pressed !== null && released !== null && pressed !== released && pressed.replace(ANYWAY, '') === released.replace(ANYWAY, '');

export const slideAcross = async (page: Page, target: Locator): Promise<void> => {
  await expect(target).toBeEnabled();
  await target.hover();
  const box = await steadyBox(page, target);
  if (box === null) throw new Error('slide target is not on screen');
  const y = box.y + box.height / 2;
  await page.mouse.move(box.x + 10, y);
  const pressed = await target.getAttribute('aria-label', { timeout: 1_000 }).catch(() => null);
  await page.mouse.down();
  const caught = await target.evaluate((slide, pointer) => slide.hasPointerCapture(pointer), MOUSE_POINTER, { timeout: 1_000 }).catch(() => true);
  await page.mouse.move(box.x + box.width - 4, y, { steps: 20 });
  await page.mouse.up();
  const released = await target.getAttribute('aria-label', { timeout: 1_000 }).catch(() => null);
  return !caught || warnedMidSlide(pressed, released) ? slideAcross(page, target) : undefined;
};

export const flyNow = async (page: Page) => {
  const later = page.getByRole('button', { name: 'Fly now, finish loading in background' });
  await expect(async () => {
    if (await later.isVisible()) await later.click({ timeout: 2_000 });
    await expect(later).toBeHidden({ timeout: 2_000 });
  }).toPass();
};

export const vehicleStatus = async (page: Page) => {
  await page.getByRole('button', { name: /^Not fully ready$|^Ready to fly$|^Armed$|^Flying$/ }).first().click();
  return page.getByRole('dialog');
};

export const mapOrMapButton = (page: Page): Locator => page.getByRole('region', { name: 'Map' }).or(page.getByRole('button', { name: 'Map', exact: true })).first();

export const showMapFull = async (page: Page) => {
  const map = page.getByRole('region', { name: 'Map' });
  const toMap = page.getByRole('button', { name: /^(Map|Mini-map)$/ }).first();
  const full = async () => (await map.count()) > 0 && ((await map.boundingBox())?.width ?? 0) >= (page.viewportSize()?.width ?? 0) * 0.9;
  await expect(async () => {
    if (!(await full()) && (await toMap.count()) > 0) await toMap.click();
    expect(await full()).toBe(true);
  }).toPass({ timeout: 20_000 });
};

export const mapMenuAt = async (page: Page, dx: number, dy: number) => {
  const box = await page.getByRole('region', { name: 'Map' }).boundingBox();
  if (box === null) throw new Error('map is not on screen');
  await expect(async () => {
    await page.mouse.click(box.x + box.width / 2 + dx, box.y + box.height / 2 + dy, { button: 'right' });
    await expect(page.getByText(/^Lat: /)).toBeVisible({ timeout: 2_000 });
  }).toPass({ timeout: 45_000 });
};

export const armPayload = async (page: Page) => {
  const strip = page.getByRole('group', { name: 'Payload', exact: true });
  const armed = strip.getByRole('button', { name: 'Payload armed, click to make safe' });
  await expect(strip.getByRole('button', { name: /^Payload (safe, click to arm|armed, click to make safe)$/ })).toBeVisible();
  if ((await armed.count()) === 0) await strip.getByRole('button', { name: 'Payload safe, click to arm' }).click();
  await expect(armed).toBeVisible();
};

export const dropFromStrip = async (page: Page, bay: number | null = null) => {
  const strip = page.getByRole('group', { name: 'Payload', exact: true });
  await armPayload(page);
  if (bay !== null) await strip.getByRole('button', { name: new RegExp(`^Bay ${bay}, loaded`) }).click();
  await strip.getByRole('button', { name: /^Drop \d/ }).click();
};

export const takeOff = async (page: Page, height: string) => {
  await page.getByRole('button', { name: 'Take off' }).click();
  await page.getByRole('button', { name: height, exact: true }).click();
  await slideAcross(page, page.getByRole('button', { name: /^Slide to take off( anyway)? · / }));
};

export const itemCount = async (page: Page): Promise<number> => {
  const [listed] = await page.getByText(/^\d+ items$/).allTextContents();
  return listed === undefined ? 0 : Number.parseInt(listed, 10);
};

export const tapPlanMission = async (page: Page) => {
  const box = await page.getByRole('region', { name: 'Map' }).boundingBox();
  if (box === null) throw new Error('plan map is not on screen');
  const [x, y] = [box.x + box.width * 0.6, box.y + box.height * 0.5];
  await [[0, 0], [80, -60], [160, 0], [80, 80]].reduce(
    (done, [dx, dy]) => done.then(async () => {
      const before = await itemCount(page);
      await expect(page.getByRole('region', { name: 'Map' })).toBeVisible();
      await expect(async () => {
        await page.mouse.click(x + dx!, y + dy!);
        await expect.poll(() => itemCount(page), { timeout: 10_000 }).toBeGreaterThan(before);
      }).toPass({ timeout: 30_000 });
    }),
    Promise.resolve(),
  );
};

export const openAircraftSetup = async (page: Page) => {
  await openMenu(page, 'Setup');
  await expect(page.getByRole('button', { name: /^Sensors/ })).toBeVisible({ timeout: 60_000 });
};

export const openPlanAtVehicle = async (page: Page) => {
  await page.addLocatorHandler(page.getByRole('dialog').filter({ hasText: 'Aircraft connected' }), (prompt) => prompt.getByRole('button', { name: 'Keep my route' }).click());
  await openMenu(page, 'Plan');
  await expect(page.getByText('Click the map to add a waypoint')).toBeVisible();
  await expect(page.locator('span.text-label-small').getByText(/^\d+ (m|km)$/)).toHaveText(/^\d+ m$/, { timeout: 60_000 });
};

export const actionOrder = (page: Page): Locator => page.getByRole('list', { name: 'Actions in order' }).getByRole('listitem');

export const actionGrip = (page: Page, title: string): Locator => page.getByRole('button', { name: `Reorder ${title}`, exact: true });

export const centreOf = async (target: Locator): Promise<{ x: number; y: number }> => {
  const box = await target.boundingBox();
  if (box === null) throw new Error('the row is not on screen');
  return { x: Math.round(box.x + box.width / 2), y: Math.round(box.y + box.height / 2) };
};

const pathBetween = (from: { x: number; y: number }, to: { x: number; y: number }, steps: number): readonly { x: number; y: number }[] =>
  [...Array(steps).keys()].map((at) => ({ x: Math.round(from.x + ((to.x - from.x) * (at + 1)) / steps), y: Math.round(from.y + ((to.y - from.y) * (at + 1)) / steps) }));

export const mouseDragAction = async (page: Page, title: string, onto: Locator, { whileHeld = async () => undefined }: { whileHeld?: () => Promise<void> } = {}): Promise<void> => {
  const from = await centreOf(actionGrip(page, title));
  const to = await centreOf(onto);
  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  await expect(actionGrip(page, title), 'the row lifts as soon as the grip is pressed and moved').toHaveAttribute('aria-pressed', 'true');
  await pathBetween(from, { x: from.x, y: to.y }, 12).reduce((done, at) => done.then(() => page.mouse.move(at.x, at.y)), Promise.resolve());
  await whileHeld();
  await page.mouse.up();
};

export const touchDragAction = async (page: Page, title: string, row: Locator, onto: Locator): Promise<void> => {
  const finger = await page.context().newCDPSession(page);
  await finger.send('Emulation.setTouchEmulationEnabled', { enabled: true });
  const from = await centreOf(row);
  const to = await centreOf(onto);
  const touch = async (type: 'touchStart' | 'touchMove' | 'touchEnd', points: readonly { x: number; y: number }[]): Promise<void> => {
    await finger.send('Input.dispatchTouchEvent', { type, touchPoints: [...points] });
  };
  await touch('touchStart', [from]);
  await expect(actionGrip(page, title), 'a held finger lifts the whole row').toHaveAttribute('aria-pressed', 'true');
  await pathBetween(from, { x: from.x, y: to.y }, 12).reduce((done, at) => done.then(() => touch('touchMove', [at])), Promise.resolve());
  await touch('touchEnd', []);
};

export const keyboardMoveAction = async (page: Page, title: string, presses: readonly ('ArrowUp' | 'ArrowDown')[], finish: 'Space' | 'Escape' = 'Space'): Promise<void> => {
  const grip = actionGrip(page, title);
  await grip.focus();
  await page.keyboard.press('Space');
  await expect(grip).toHaveAttribute('aria-pressed', 'true');
  const live = page.locator('[id^="DndLiveRegion"]');
  await expect(live, 'the lifted row has been measured and says where it would land').toHaveText(/would be action/);
  await presses.reduce(
    (done, key) =>
      done.then(async () => {
        const before = await live.innerText();
        await page.keyboard.press(key);
        await expect(live, `${key} moves ${title} one place and says where it would land`).not.toHaveText(before);
      }),
    Promise.resolve(),
  );
  await page.keyboard.press(finish);
};

export const savedMission = (page: Page): Promise<readonly number[]> =>
  page.evaluate(
    async ([bridge, files, path]) => {
      const { Qgc } = (await import(bridge)) as { Qgc: { invokeResult: (path: string, ...args: unknown[]) => Promise<unknown> } };
      const { stagedFile } = (await import(files)) as { stagedFile: (path: string) => Promise<File | null> };
      await Qgc.invokeResult('plan.saveToFile', path);
      const text = (await (await stagedFile(path))?.text()) ?? '{}';
      const plan = JSON.parse(text) as { mission?: { items?: { command?: number; TransectStyleComplexItem?: unknown }[] } };
      return (plan.mission?.items ?? []).map((item) => item.command ?? -1);
    },
    ['/src/bridge/Qgc.ts', '/src/core/files.ts', '/e2e-order.plan'] as const,
  );

export const test = base.extend<{ app: Page; vehicle: Page }>({
  app: async ({ page }, use) => {
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await openApp(page);
    await use(page);
    expect(errors, 'uncaught page errors').toEqual([]);
  },
  vehicle: async ({ app }, use) => {
    await connectMock(app);
    await use(app);
  },
});

export { expect };

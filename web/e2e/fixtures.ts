import { test as base, expect, type Locator, type Page } from '@playwright/test';

const passFirstRun = async (page: Page) => {
  const welcome = page.getByRole('dialog').filter({ has: page.getByRole('heading', { name: 'Welcome to Aircast' }) });
  await welcome.getByRole('button', { name: 'Continue' }).click();
  await expect(welcome).toBeHidden();
};

export const openApp = async (page: Page, { firstRun = true } = {}) => {
  await page.goto('/');
  if (!firstRun) return;
  await passFirstRun(page);
  await expect(page.getByRole('region', { name: 'Map' })).toBeVisible();
};

export const openMenu = async (page: Page, item: string) => {
  await page.getByRole('button', { name: 'Menu' }).click();
  await page.getByRole('menuitem', { name: item }).click();
};

export type MockOption = 'Enable camera' | 'Enable gimbal' | 'Enable proximity sensors' | 'Send status text + voice';

export const connectMock = async (page: Page, options: readonly MockOption[] = []) => {
  await page.getByRole('button', { name: /How to connect/ }).click();
  await page.getByRole('button', { name: 'Add link…' }).click();
  await page.getByRole('button', { name: /^Simulated vehicle/ }).click();
  await options.reduce((done, name) => done.then(() => page.getByRole('checkbox', { name }).check()), Promise.resolve());
  await page.getByRole('button', { name: 'Start', exact: true }).click();
  await expect(page.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 30_000 });
};

export const flightMode = (page: Page): Locator => page.getByRole('button', { name: /Change flight mode$/ }).first();

export const telemetry = (page: Page, label: string): Locator => page.getByText(label, { exact: true }).locator('..').filter({ hasText: /\d/ });

export const slideAcross = async (page: Page, target: Locator) => {
  await target.hover();
  const box = await target.boundingBox();
  if (box === null) throw new Error('slide target is not on screen');
  const y = box.y + box.height / 2;
  await page.mouse.move(box.x + 10, y);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width - 4, y, { steps: 20 });
  await page.mouse.up();
};

export const vehicleStatus = async (page: Page) => {
  await page.getByRole('button', { name: /^Not fully ready$|^Ready to fly$|^Armed$/ }).first().click();
  return page.getByRole('dialog');
};

export const showMapFull = async (page: Page) => {
  const map = page.getByRole('region', { name: 'Map' });
  const full = async () => ((await map.boundingBox())?.width ?? 0) >= (page.viewportSize()?.width ?? 0) * 0.9;
  await expect(async () => {
    if (!(await full())) await page.getByRole('button', { name: /^(Map|Mini-map)$/ }).first().click();
    expect(await full()).toBe(true);
  }).toPass({ timeout: 20_000 });
};

export const mapMenuAt = async (page: Page, dx: number, dy: number) => {
  const box = await page.getByRole('region', { name: 'Map' }).boundingBox();
  if (box === null) throw new Error('map is not on screen');
  await expect(async () => {
    await page.mouse.click(box.x + box.width / 2 + dx, box.y + box.height / 2 + dy, { button: 'right' });
    await expect(page.getByText(/^Lat: /)).toBeVisible({ timeout: 1_000 });
  }).toPass({ timeout: 20_000 });
};

export const takeOff = async (page: Page, height: string) => {
  await page.getByRole('button', { name: 'Take off' }).click();
  await page.getByRole('button', { name: height, exact: true }).click();
  await slideAcross(page, page.getByRole('button', { name: /^Slide to take off/ }));
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
      await page.mouse.click(x + dx!, y + dy!);
      await expect.poll(() => itemCount(page)).toBeGreaterThan(before);
    }),
    Promise.resolve(),
  );
};

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

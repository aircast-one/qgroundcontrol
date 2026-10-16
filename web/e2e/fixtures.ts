import { test as base, expect, type Page } from '@playwright/test';

const passFirstRun = async (page: Page) => {
  const welcome = page.getByRole('dialog').filter({ has: page.getByRole('heading', { name: 'Welcome to Aircast' }) });
  await welcome.getByRole('button', { name: 'Continue' }).click();
  await expect(welcome).toBeHidden();
};

export const openApp = async (page: Page, { firstRun = true } = {}) => {
  await page.goto('/');
  await expect(page.getByRole('region', { name: 'Map' })).toBeVisible();
  if (firstRun) await passFirstRun(page);
};

export const openMenu = async (page: Page, item: string) => {
  await page.getByRole('button', { name: 'Menu' }).click();
  await page.getByRole('menuitem', { name: item }).click();
};

export const test = base.extend<{ app: Page }>({
  app: async ({ page }, use) => {
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await openApp(page);
    await use(page);
    expect(errors, 'uncaught page errors').toEqual([]);
  },
});

export { expect };

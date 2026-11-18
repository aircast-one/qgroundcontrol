import type { Page } from '@playwright/test';
import { closeSettings, expect, test } from './fixtures';

const RELAY = 'https://localhost:18443';

const cloudForm = async (app: Page) => {
  await app.getByRole('button', { name: /How to connect/ }).click();
  await app.getByRole('button', { name: 'Add a link' }).click();
  await app.getByRole('button', { name: /^Aircast Cloud/ }).click();
  const form = app.locator('div').filter({ hasText: 'A backup link to the aircraft through your Aircast account.' }).filter({ hasNotText: 'Connect to an Aircast device' }).filter({ has: app.getByRole('button', { name: 'Connect', exact: true }) }).last();
  await form.getByRole('textbox', { name: 'Account server' }).fill(RELAY);
  await form.getByRole('textbox', { name: 'Device', exact: true }).fill('e2e-drone');
  return form;
};

test('an Aircast Cloud link signs in and reaches the vehicle over WebTransport', async ({ app, request }) => {
  const heard = async () => ((await (await request.get(`${RELAY}/stats`, { ignoreHTTPSErrors: true })).json()) as { gcsHeartbeats: number }).gcsHeartbeats;
  const before = await heard();
  const form = await cloudForm(app);
  await form.getByRole('button', { name: 'Sign in' }).click();
  await expect(form.getByRole('button', { name: 'Sign out' })).toBeVisible({ timeout: 30_000 });
  await form.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(app.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });
  await expect(app.getByText('20.0 m', { exact: true })).toBeVisible();
  await expect.poll(heard, { timeout: 15_000 }).toBeGreaterThan(before);
});

test('a cloud link that needs a sign-in offers Sign in in the connection sheet, and signing in there connects it', async ({ app }) => {
  const form = await cloudForm(app);
  await form.getByRole('button', { name: 'Connect', exact: true }).click();
  await closeSettings(app);
  await app.getByRole('button', { name: /How to connect/ }).click();
  const sheet = app.getByRole('dialog');
  await expect(sheet.getByText('Sign in to your Aircast account to use the cloud backup link.')).toBeVisible({ timeout: 15_000 });
  await sheet.getByText('Sign in', { exact: true }).click();
  await expect(app.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });
});

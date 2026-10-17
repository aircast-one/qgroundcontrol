import { expect, test } from './fixtures';

const RELAY = 'https://localhost:18443';

test('an Aircast Cloud link signs in and reaches the vehicle over WebTransport', async ({ app, request }) => {
  const heard = async () => ((await (await request.get(`${RELAY}/stats`, { ignoreHTTPSErrors: true })).json()) as { gcsHeartbeats: number }).gcsHeartbeats;
  const before = await heard();
  await app.getByRole('button', { name: /How to connect/ }).click();
  await app.getByRole('button', { name: 'Add link…' }).click();
  await app.getByRole('button', { name: /^Aircast Cloud/ }).click();
  await app.getByRole('textbox', { name: 'Account server' }).fill(RELAY);
  await app.getByRole('textbox', { name: 'Device' }).fill('e2e-drone');
  await app.getByRole('button', { name: 'Sign in' }).click();
  await expect(app.getByRole('button', { name: 'Sign out' })).toBeVisible({ timeout: 30_000 });
  await app.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(app.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });
  await expect(app.getByText('20.0m', { exact: true })).toBeVisible();
  await expect.poll(heard, { timeout: 15_000 }).toBeGreaterThan(before);
});

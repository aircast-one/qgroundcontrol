import { connectMock, expect, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('an enforced preflight checklist pops up on connect, ticks off and resets', async ({ app }) => {
  await app.getByRole('button', { name: 'Settings', exact: true }).click();
  await app.getByRole('tablist').getByRole('tab', { name: 'General' }).click();
  await app.getByRole('switch', { name: 'Use preflight checklist' }).click();
  await app.getByRole('switch', { name: 'Enforce preflight checklist' }).click();
  await app.getByRole('button', { name: 'Close settings' }).click();
  await connectMock(app);
  const checklist = app.getByRole('dialog').filter({ hasText: 'Pre-flight checklist' });
  await expect(checklist).toBeVisible({ timeout: 30_000 });
  await expect(checklist.getByText(/^9 of 9 left to check/)).toBeVisible();
  await ['Props mounted and secured?', 'Battery connector firmly plugged?', 'Receiving signal. Perform range test & confirm.'].reduce(
    (done, check) => done.then(() => checklist.getByRole('checkbox', { name: check }).click()),
    Promise.resolve(),
  );
  await expect(checklist.getByText(/^6 of 9 left to check/)).toBeVisible();
  await expect(checklist.getByRole('checkbox', { name: 'Props mounted and secured?' })).toBeChecked();
  await checklist.getByRole('button', { name: 'Reset' }).click();
  await expect(checklist.getByText(/^9 of 9 left to check/)).toBeVisible();
  await checklist.getByRole('button', { name: 'Close' }).click();
  await expect(checklist).toBeHidden();
});

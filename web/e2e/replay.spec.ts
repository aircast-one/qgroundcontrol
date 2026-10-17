import { type Page } from '@playwright/test';
import { expect, test } from './fixtures';
import { flightLog } from './tlog';

const HOME = { lat: 47.3977, lon: 8.5456 };

const showReplayBar = async (page: Page) => {
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('button', { name: 'Search settings' }).click();
  await page.getByRole('textbox').first().fill('replay');
  const bar = page.getByRole('switch', { name: /replay/i });
  await bar.click();
  await expect(bar).toBeChecked();
  await page.getByRole('button', { name: 'Close settings' }).click();
};

const replayLink = async (page: Page) => {
  await page.getByRole('button', { name: /How to connect/ }).click();
  await page.getByRole('button', { name: 'Add link…' }).click();
  await page.getByRole('button', { name: /^Replay a flight log/ }).click();
  const chooser = page.waitForEvent('filechooser');
  await page.getByRole('button', { name: 'Choose a log file' }).click();
  await (await chooser).setFiles({ name: 'flight.tlog', mimeType: 'application/octet-stream', buffer: flightLog(30, HOME) });
  await page.getByRole('button', { name: 'Connect', exact: true }).click();
};

test('a flight log replays as a connected vehicle', async ({ app }) => {
  await replayLink(app);
  await expect(app.getByRole('button', { name: /Change flight mode/ }).first()).toBeVisible({ timeout: 60_000 });
  await expect(app.getByText('20.0m', { exact: true })).toBeVisible();
});

test('the replay bar shows progress and pauses the log', async ({ app }) => {
  await showReplayBar(app);
  await replayLink(app);
  const progress = app.getByText(/^\d\dm:\d\ds of 00m:29s$/);
  await expect(progress).toBeVisible({ timeout: 60_000 });
  const first = await progress.textContent();
  await expect.poll(() => progress.textContent(), { timeout: 30_000 }).not.toBe(first);
  await app.getByRole('button', { name: 'Pause' }).click();
  await expect(app.getByRole('button', { name: 'Play' })).toBeVisible();
});

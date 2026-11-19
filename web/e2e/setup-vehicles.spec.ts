import { connectMock, expect, openAircraftSetup, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('a browser leaves Remote support out of ArduPilot setup, since it forwards over a UDP socket', async ({ app }) => {
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await openAircraftSetup(app);
  await expect(app.getByRole('button', { name: /^Logging / })).toBeVisible();
  await expect(app.getByRole('button', { name: /^Remote support/ })).toHaveCount(0);
});

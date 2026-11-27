import { expect, test } from './fixtures';

test('Choose readings brings back the attitude and compass dials, which follow the aircraft', async ({ vehicle }) => {
  const attitude = vehicle.getByRole('img', { name: /^Attitude, pitch -?\d+°, roll -?\d+°$/ });
  await expect(attitude, 'they start hidden, as an option').toHaveCount(0);
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Choose readings').click();
  const chooser = vehicle.getByRole('dialog', { name: 'Choose readings' });
  await chooser.getByText('Attitude and compass', { exact: true }).click();
  await vehicle.keyboard.press('Escape');
  await expect(attitude).toBeVisible();
  const box = (await attitude.boundingBox())!;
  const compass = vehicle.getByRole('img', { name: /^Heading / }).filter({ visible: true }).last();
  await expect(compass).toBeVisible();
  const beside = (await compass.boundingBox())!;
  expect(Math.abs(beside.y - box.y), 'the compass sits beside the horizon, like the QGC pair').toBeLessThan(4);
  await vehicle.getByRole('button', { name: 'Menu', exact: true }).click();
  await vehicle.getByRole('dialog').getByText('Choose readings').click();
  await vehicle.getByRole('dialog', { name: 'Choose readings' }).getByText('Attitude and compass', { exact: true }).click();
  await vehicle.keyboard.press('Escape');
  await expect(attitude, 'and the same row hides them again').toHaveCount(0);
});

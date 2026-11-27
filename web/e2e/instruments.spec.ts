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

test('the small picture in the corner is dragged anywhere on screen, a tap still swaps the map and the camera, and the other picture takes its place', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  const mapButton = vehicle.getByRole('button', { name: 'Map', exact: true });
  await expect(mapButton).toBeVisible({ timeout: 30_000 });
  const home = (await mapButton.boundingBox())!;
  await vehicle.mouse.move(home.x + home.width / 2, home.y + home.height / 2);
  await vehicle.mouse.down();
  await vehicle.mouse.move(home.x + home.width / 2 + 300, home.y + home.height / 2 - 250, { steps: 12 });
  await vehicle.mouse.up();
  const moved = (await mapButton.boundingBox())!;
  expect([Math.round(moved.x - home.x), Math.round(moved.y - home.y)], 'dropped where the pointer let go, and the drop did not swap the views').toEqual([300, -250]);
  await mapButton.click();
  const video = vehicle.getByRole('button', { name: 'Video', exact: true });
  await expect(video, 'a tap swaps: the map fills the screen and the camera shrinks into the corner').toBeVisible({ timeout: 30_000 });
  const camera = (await video.boundingBox())!;
  expect(camera.y, 'the camera picture comes up where the map button was left, not back at the bottom').toBeLessThan(home.y - 200);
  await vehicle.mouse.move(camera.x + camera.width / 2, camera.y + camera.height / 2);
  await vehicle.mouse.down();
  await vehicle.mouse.move(camera.x + camera.width / 2 + 120, camera.y + camera.height / 2 + 60, { steps: 12 });
  await vehicle.mouse.up();
  const dragged = (await video.boundingBox())!;
  expect([Math.round(dragged.x - camera.x), Math.round(dragged.y - camera.y)]).toEqual([120, 60]);
  await video.click();
  await expect(mapButton, 'and a tap on the camera swaps back').toBeVisible();
});

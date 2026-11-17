import { connectMock, expect, flightMode, mapMenuAt, showMapFull, slideAcross, takeOff, telemetry, test } from './fixtures';

test.describe.configure({ timeout: 120_000 });

test('on the ground a click on the map teaches the right-click, and the map menu offers a new home position', async ({ vehicle }) => {
  await showMapFull(vehicle);
  const box = (await vehicle.getByRole('region', { name: 'Map' }).boundingBox())!;
  await expect(async () => {
    await vehicle.mouse.click(box.x + box.width / 2 - 200, box.y + box.height / 2 + 100);
    await expect(vehicle.getByText('Right-click the map to fly there')).toBeVisible({ timeout: 2_000 });
  }).toPass({ timeout: 45_000 });
  await mapMenuAt(vehicle, 200, -100);
  await expect(vehicle.getByRole('button', { name: 'Set home here' })).toBeVisible();
  await expect(vehicle.getByText(/^Lat: -?\d+\.\d{6}$/)).toBeVisible();
});

test('in flight the map menu flies to the clicked point', async ({ vehicle }) => {
  await takeOff(vehicle, '10 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await showMapFull(vehicle);
  await mapMenuAt(vehicle, 200, -100);
  await vehicle.getByRole('button', { name: 'Go to location' }).click();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to go to location' }));
  await expect(vehicle.getByText('Going to point')).toBeVisible();
  await expect(vehicle.getByText(/^Arrived/)).toBeVisible({ timeout: 90_000 });
  await expect(vehicle.getByText(/transfer failed|did not take the go-to point/)).toBeHidden();
});

test('in flight an ArduCopter flies to the clicked point without saying the transfer failed', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await takeOff(vehicle, '10 m');
  await expect(telemetry(vehicle, 'H')).toHaveText(/^H(9\.[5-9]|10\.[0-4]) m$/, { timeout: 60_000 });
  await showMapFull(vehicle);
  await mapMenuAt(vehicle, 200, -100);
  await vehicle.getByRole('button', { name: 'Go to location' }).click();
  const slide = vehicle.getByRole('button', { name: 'Slide to go to location' });
  await expect(async () => {
    if (await slide.isVisible()) await slideAcross(vehicle, slide);
    await expect(slide).toBeHidden({ timeout: 2_000 });
  }).toPass({ timeout: 20_000 });
  await expect(vehicle.getByText(/^Arrived/)).toBeVisible({ timeout: 90_000 });
  await expect(vehicle.getByText(/transfer failed|did not take the go-to point/)).toBeHidden();
});

test('a go-to point does not outlive the motors stopping', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await takeOff(vehicle, '10 m');
  await expect(telemetry(vehicle, 'H')).toHaveText(/^H(9\.[5-9]|10\.[0-4]) m$/, { timeout: 60_000 });
  await showMapFull(vehicle);
  await mapMenuAt(vehicle, 200, -100);
  await vehicle.getByRole('button', { name: 'Go to location' }).click();
  const slide = vehicle.getByRole('button', { name: 'Slide to go to location' });
  await expect(async () => {
    if (await slide.isVisible()) await slideAcross(vehicle, slide);
    await expect(slide).toBeHidden({ timeout: 2_000 });
  }).toPass({ timeout: 20_000 });
  await expect(vehicle.getByText(/^Going to point/)).toBeVisible();
  await vehicle.getByRole('group', { name: 'Flight actions' }).getByRole('button', { name: 'Kill motors', exact: true }).click();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to emergency stop' }));
  await expect(vehicle.getByRole('button', { name: /^Not fully ready$|^Ready to fly$/ }).first()).toBeVisible({ timeout: 30_000 });
  await expect(vehicle.getByText(/^(Going to point|Arrived)/)).toHaveCount(0);
});

test('a go-to offers a height that starts at the current height, and its card keeps clear of the point', async ({ vehicle }) => {
  await takeOff(vehicle, '10 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await showMapFull(vehicle);
  await mapMenuAt(vehicle, 400, -100);
  await vehicle.getByRole('button', { name: 'Go to location' }).click();
  const card = vehicle.getByRole('dialog', { name: 'Map actions' });
  await expect(card.getByText(/\(current height\)$/)).toBeVisible();
  const box = await card.boundingBox();
  expect((box?.x ?? 0) + (box?.width ?? 0), 'a point on the right puts the card on the left').toBeLessThan((vehicle.viewportSize()?.width ?? 0) / 2);
  await card.getByRole('button', { name: /increase|\+/i }).first().click();
  await expect(card.getByText(/ · at \d+ (m|ft)$/)).toBeVisible();
  await expect(card.getByText(/\(current height\)$/)).toBeHidden();
});

test('a stepped go-to height is the height commanded, even when the slide follows at once', async ({ vehicle }) => {
  await takeOff(vehicle, '10 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await showMapFull(vehicle);
  await vehicle.evaluate(() => {
    const record = window as unknown as { gotoSent: readonly { height?: number }[] };
    record.gotoSent = [];
    const post = Worker.prototype.postMessage;
    Worker.prototype.postMessage = function posted(this: Worker, message: unknown, transfer?: Transferable[] | StructuredSerializeOptions) {
      const sent = message as { call?: string; args?: readonly unknown[] };
      if (sent.call === 'invoke' && sent.args?.[0] === 'vehicle.guidedModeGotoLocation') record.gotoSent = [...record.gotoSent, (JSON.parse(String(sent.args[1])) as { height?: number }[])[0] ?? {}];
      return Reflect.apply(post, this, [message, transfer]);
    };
  });
  await mapMenuAt(vehicle, 200, -100);
  await vehicle.getByRole('button', { name: 'Go to location' }).click();
  const card = vehicle.getByRole('dialog', { name: 'Map actions' });
  const shown = card.getByRole('button', { name: /^Height: \d+ m$/ });
  const before = Number((await shown.getAttribute('aria-label'))?.replace(/\D/g, ''));
  const increase = await card.getByRole('button', { name: 'Increase Height' }).boundingBox();
  const slide = await card.getByRole('button', { name: 'Slide to go to location' }).boundingBox();
  if (increase === null || slide === null) throw new Error('the go-to card is not on screen');
  await vehicle.mouse.click(increase.x + increase.width / 2, increase.y + increase.height / 2);
  await vehicle.mouse.move(slide.x + 10, slide.y + slide.height / 2);
  await vehicle.mouse.down();
  await vehicle.mouse.move(slide.x + slide.width - 4, slide.y + slide.height / 2, { steps: 4 });
  await vehicle.mouse.up();
  await expect(vehicle.getByText(/^Going to point/)).toBeVisible();
  const commanded = await vehicle.evaluate(() => (window as unknown as { gotoSent: { height?: number }[] }).gotoSent.map((it) => it.height ?? null));
  expect(commanded).toEqual([before + 5]);
  await expect(telemetry(vehicle, 'H')).toHaveText(new RegExp(`^H(${before + 4}\\.[5-9]|${before + 5}\\.[0-4]) m$`), { timeout: 90_000 });
});

test('dragging the map stops it following the vehicle until the follow button brings it back', async ({ vehicle }) => {
  await showMapFull(vehicle);
  const box = (await vehicle.getByRole('region', { name: 'Map' }).boundingBox())!;
  const follow = vehicle.getByRole('button', { name: /^Follow the vehicle/ });
  await expect(follow).toBeHidden();
  await expect(async () => {
    await vehicle.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await vehicle.mouse.down();
    await vehicle.mouse.move(box.x + 60, box.y + 80, { steps: 12 });
    await vehicle.mouse.up();
    await expect(follow).toBeVisible({ timeout: 2_000 });
  }).toPass({ timeout: 45_000 });
  await follow.click();
  await expect(follow).toBeHidden();
});

test('in flight the map menu offers an orbit with a radius range', async ({ vehicle }) => {
  await takeOff(vehicle, '10 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await showMapFull(vehicle);
  await mapMenuAt(vehicle, 150, -80);
  await vehicle.getByRole('button', { name: 'Orbit at location' }).click();
  await expect(vehicle.getByText('Orbit the vehicle around the specified location')).toBeVisible();
  await expect(vehicle.getByText(/^[\d.]+ to [\d.]+ m$/)).toBeVisible();
  await expect(vehicle.getByRole('button', { name: 'Slide to orbit' })).toBeVisible();
  await vehicle.getByRole('button', { name: 'Cancel' }).click();
  await expect(vehicle.getByRole('button', { name: 'Slide to orbit' })).toBeHidden();
});

test('in flight the map menu flies an orbit around the clicked point', async ({ vehicle }) => {
  await takeOff(vehicle, '10 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await showMapFull(vehicle);
  await mapMenuAt(vehicle, 150, -80);
  await vehicle.getByRole('button', { name: 'Orbit at location' }).click();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to orbit' }));
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Orbit/, { timeout: 30_000 });
});

import type { Locator, Page } from '@playwright/test';
import { WHEP_STREAM, addVideoSource, chooseCameraJoystick, connectMock, expect, flightMode, flyNow, showMapFull, slideAcross, takeOff, telemetry, test } from './fixtures';

const LONG_PRESS_MS = 400;
const CAMERA_PANEL_OFFSET = 'aircast.fly-overlay-layout.OverlayRigOffset-cameraControl@landscape';

const centre = async (target: Locator): Promise<readonly [number, number]> => {
  const box = await target.boundingBox();
  if (box === null) throw new Error('target is not on screen');
  return [box.x + box.width / 2, box.y + box.height / 2];
};

const editBar = (page: Page): Locator => page.getByRole('button', { name: 'Layout options' });

const holdUntilEditing = async (page: Page, target: Locator) => {
  const [x, y] = await centre(target);
  await page.mouse.move(x, y);
  await page.mouse.down();
  await expect(editBar(page)).toBeVisible();
  await page.mouse.up();
};

const pastLongPress = (page: Page, since: number) => page.waitForFunction(([from, wait]) => performance.now() - from > wait, [since, LONG_PRESS_MS * 2] as const);

const storedCameraPanel = (page: Page) => page.evaluate((key) => localStorage.getItem(key), CAMERA_PANEL_OFFSET);

const gimbalAim = (page: Page) =>
  page.evaluate(
    async (module) =>
      (((await import(module)) as { Qgc: { get: (path: string) => Promise<unknown> } }).Qgc.get('view.gimbal') as Promise<{ gimbals?: { pitch: number | null; bodyYaw: number | null }[] } | null>).then((it) => ({ pitch: it?.gimbals?.[0]?.pitch ?? null, yaw: it?.gimbals?.[0]?.bodyYaw ?? null })),
    '/src/bridge/Qgc.ts',
  );

const withCamera = async (page: Page) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await connectMock(page, ['Enable camera', 'Enable gimbal']);
  await expect(page.getByRole('button', { name: 'Start recording' })).toBeVisible();
};

const bottomOf = async (target: Locator): Promise<number> => {
  const box = await target.boundingBox();
  if (box === null) throw new Error('target is not on screen');
  return box.y + box.height;
};

const editLayout = async (page: Page) => {
  await page.getByRole('button', { name: 'Menu', exact: true }).click();
  await page.getByRole('dialog').getByText('Edit layout').click();
  await expect(editBar(page)).toBeVisible();
};

const aloft = (target: Locator) =>
  target.evaluate((node) => {
    const chain = (at: Element | null): Element[] => (at === null ? [] : [at, ...chain(at.parentElement)]);
    const around = chain(node);
    const jiggling = document
      .getAnimations()
      .filter((it): it is CSSAnimation => it instanceof CSSAnimation && it.animationName === 'aircast-overlay-jiggle' && it.playState === 'running')
      .some((it) => it.effect instanceof KeyframeEffect && it.effect.target !== null && around.includes(it.effect.target));
    return { scale: Math.max(...around.map((at) => Number(getComputedStyle(at).scale) || 1)), jiggling };
  });

const liftsWhileHeld = async (page: Page, target: Locator) => {
  await expect.poll(() => aloft(target)).toEqual({ scale: 1, jiggling: true });
  const [x, y] = await centre(target);
  await page.mouse.move(x, y);
  await page.mouse.down();
  await expect.poll(() => aloft(target), 'held, it grows and stops jiggling').toEqual({ scale: 1.1, jiggling: false });
  await page.mouse.up();
  await expect.poll(() => aloft(target), 'let go, it settles back and jiggles again').toEqual({ scale: 1, jiggling: true });
};

const dragCameraPanel = async (page: Page, dx: number, dy: number) => {
  await editLayout(page);
  const [x, y] = await centre(page.getByRole('button', { name: 'Start recording' }));
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + dx, y + dy, { steps: 20 });
  await page.mouse.up();
  await page.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(page)).toBeHidden();
};

test('holding the shutter starts layout editing without recording, and a tap still records', async ({ app }) => {
  await withCamera(app);
  const shutter = app.getByRole('button', { name: 'Start recording' });
  await holdUntilEditing(app, shutter);
  await app.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(app)).toBeHidden();
  await expect(shutter, 'the release that ended the hold did not press the shutter').toBeVisible();
  await shutter.click();
  await expect(app.getByRole('button', { name: 'Stop recording' }), 'one tap, one toggle: the hold never toggled it').toBeVisible();
});

test('after a hold on an indicator inside the status pill, the keyboard still opens that indicator', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  const battery = vehicle.getByRole('button', { name: /^Battery \d+%/ }).first();
  await holdUntilEditing(vehicle, battery);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(vehicle)).toBeHidden();
  await battery.focus();
  await vehicle.keyboard.press('Enter');
  await expect(vehicle.getByRole('dialog').getByText(/^Battery 1/)).toBeVisible();
});

test('only widgets without layout widgets inside them jiggle, so the status pill does not swing as one slab', async ({ vehicle }) => {
  await editLayout(vehicle);
  const jiggling = () =>
    vehicle.evaluate(() => {
      const targets = document
        .getAnimations()
        .filter((it): it is CSSAnimation => it instanceof CSSAnimation && it.animationName === 'aircast-overlay-jiggle')
        .map((it) => it.effect)
        .filter((it): it is KeyframeEffect => it instanceof KeyframeEffect)
        .map((it) => it.target)
        .filter((it): it is Element => it !== null);
      return { count: targets.length, nested: targets.filter((it) => targets.some((other) => other !== it && other.contains(it))).length };
    });
  await expect.poll(async () => (await jiggling()).count).toBeGreaterThan(1);
  expect((await jiggling()).nested, 'a jiggling widget inside another jiggling widget').toBe(0);
});

const EDIT_CONTROLS = ['Layout options', 'Add reading', 'Done'];

const THIN_BAR = 24;

const clippedBadges = (page: Page) =>
  page.evaluate(() =>
    [...document.querySelectorAll('[aria-label=Hide]')]
      .filter((badge) => {
        const widget = badge.parentElement?.getBoundingClientRect();
        return widget !== undefined && badge.parentElement?.contains(document.elementFromPoint(widget.left + widget.width / 2, widget.top + widget.height / 2)) === true;
      })
      .filter((badge) => {
        const box = badge.getBoundingClientRect();
        const [x, y] = [box.left + box.width / 2, box.top + box.height / 2];
        const edges = [
          [x, box.top + 1.5],
          [x, box.bottom - 1.5],
          [box.left + 1.5, y],
          [box.right - 1.5, y],
        ];
        return edges.some(([px, py]) => !badge.contains(document.elementFromPoint(px, py)));
      })
      .map((badge) => badge.parentElement?.querySelector('[aria-label]:not([aria-label=Hide])')?.getAttribute('aria-label') ?? badge.parentElement?.textContent?.trim() ?? '?'),
  );

const stillWhileEditing = (page: Page) =>
  page.evaluate(([controls, thinBar]) => {
    const jiggling = document
      .getAnimations()
      .filter((it): it is CSSAnimation => it instanceof CSSAnimation && it.animationName === 'aircast-overlay-jiggle')
      .map((it) => (it.effect instanceof KeyframeEffect ? it.effect.target : null))
      .filter((it): it is Element => it !== null);
    const name = (it: Element) => it.getAttribute('aria-label') ?? it.querySelector('[aria-label]')?.getAttribute('aria-label') ?? it.textContent?.trim() ?? it.tagName;
    const room = innerWidth - (document.querySelector('nav')?.getBoundingClientRect().width ?? 0);
    return [...document.querySelectorAll('button, [role=button], [role=group], [role=img], [role=region], svg:not(button svg):not([role=button] svg):not([role=img] svg)')]
      .filter((it) => it.closest('nav') === null)
      .filter((it) => {
        const box = it.getBoundingClientRect();
        return box.width > 1 && box.height > 1 && (box.width < room - 1 || box.height < thinBar);
      })
      .filter((it) => !jiggling.some((widget) => widget.contains(it)) && !jiggling.some((widget) => it.contains(widget)))
      .map(name)
      .filter((it) => !controls.includes(it));
  }, [EDIT_CONTROLS, THIN_BAR] as const);

(
  [
    ['landscape', 1280, 800],
    ['portrait', 412, 860],
  ] as const
).forEach(([orientation, width, height]) =>
  test(`in ${orientation} layout edit everything on the fly screen jiggles where it stands`, async ({ app }) => {
    await app.emulateMedia({ reducedMotion: 'no-preference' });
    await app.setViewportSize({ width, height });
    await connectMock(app, ['Enable camera', 'Enable gimbal']);
    const payload = app.getByRole('group', { name: 'Payload' });
    await expect(payload).toBeVisible();
    await expect(app.getByRole('button', { name: /^Cameras, showing/ }), 'the camera has arrived, so the screen has its final layout').toBeVisible({ timeout: 30_000 });
    const [x, y] = await centre(payload);
    await editLayout(app);
    await expect.poll(() => stillWhileEditing(app)).toEqual([]);
    await expect.poll(() => clippedBadges(app), { message: 'a hide badge cut off or under something' }).toEqual([]);
    await expect(app.getByLabel('Always shown')).toHaveCount(0);
    const [editingX, editingY] = await centre(payload);
    expect([Math.round(editingX), Math.round(editingY)], 'the payload strip stays where it was').toEqual([Math.round(x), Math.round(y)]);
  }),
);

test('while editing the layout a tap on an always-shown control does nothing', async ({ vehicle }) => {
  const [x, y] = await centre(vehicle.getByRole('group', { name: 'Flight actions' }).getByRole('button').first());
  await editLayout(vehicle);
  await vehicle.mouse.click(x, y);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(vehicle)).toBeHidden();
  await expect(vehicle.getByText('Height above launch'), 'the tap did not open the takeoff panel').toHaveCount(0);
  await expect(vehicle.getByRole('dialog')).toHaveCount(0);
});

test('the message banner hugs its text, so its jiggle swings a few pixels and not the width of the screen', async ({ app }) => {
  await connectMock(app, ['Send status text + voice']);
  const dismiss = app.getByRole('button', { name: 'Dismiss messages', exact: true });
  await expect(dismiss).toBeVisible();
  await editLayout(app);
  const banner = app.getByRole('button').filter({ has: dismiss });
  const [bannerBox, dismissBox] = await Promise.all([banner.boundingBox(), dismiss.boundingBox()]);
  if (bannerBox === null || dismissBox === null) throw new Error('the banner is not on screen');
  expect(bannerBox.x + bannerBox.width - (dismissBox.x + dismissBox.width), 'room past the dismiss button').toBeLessThan(24);
});

test('while editing the layout the video takes no input, a click on it ends editing, and it takes input again after', async ({ app }) => {
  await withCamera(app);
  const [x, y] = await app.evaluate(() => [Math.round(innerWidth * 0.4), Math.round(innerHeight * 0.6)] as const);
  await app.evaluate(([px, py]) => {
    const video = document.elementFromPoint(px, py);
    if (video === null) throw new Error('nothing under the probe point');
    Object.assign(window, { videoPresses: 0 });
    video.addEventListener('pointerdown', () => Object.assign(window, { videoPresses: (window as unknown as { videoPresses: number }).videoPresses + 1 }));
  }, [x, y] as const);
  const presses = () => app.evaluate(() => (window as unknown as { videoPresses: number }).videoPresses);
  await editLayout(app);
  await app.mouse.click(x, y);
  await expect(editBar(app), 'a click on the picture is a click on empty space').toBeHidden();
  expect(await presses(), 'the press made while editing').toBe(0);
  await app.mouse.click(x, y);
  await expect.poll(presses, { message: 'the probe point is the video' }).toBe(1);
});

(
  [
    ['landscape', 1280, 800],
    ['portrait', 412, 860],
  ] as const
).forEach(([orientation, width, height]) =>
  test(`in ${orientation} a mouse drag across the Fly screen selects no text, while editing the layout too`, async ({ app }) => {
    await app.setViewportSize({ width, height });
    await withCamera(app);
    const selectedBy = async (fromX: number, fromY: number, toX: number, toY: number) => {
      await app.mouse.move(fromX, fromY);
      await app.mouse.down();
      await app.mouse.move(toX, toY, { steps: 12 });
      await app.mouse.up();
      return app.evaluate(() => getSelection()?.toString() ?? '');
    };
    const across = () => selectedBy(8, 60, width - 8, height - 8);
    const down = () => selectedBy(width / 2, 8, width / 2, height - 8);
    expect(await across(), 'flying, corner to corner').toBe('');
    expect(await down(), 'flying, top to bottom').toBe('');
    await editLayout(app);
    expect(await across(), 'editing the layout, corner to corner').toBe('');
    expect(await down(), 'editing the layout, top to bottom').toBe('');
  }),
);

test('a widget dragged up stops below the status row, also after turning the screen and back', async ({ app }) => {
  await withCamera(app);
  const rowWidth = async () => (await app.locator('[data-status-row]').boundingBox())?.width ?? 0;
  await app.setViewportSize({ width: 412, height: 860 });
  await expect.poll(rowWidth).toBeLessThan(500);
  await app.setViewportSize({ width: 1280, height: 800 });
  await expect.poll(rowWidth).toBeGreaterThan(500);
  await dragCameraPanel(app, 0, -1000);
  const row = await bottomOf(app.locator('[data-status-row]'));
  const zoom = await app.getByRole('button', { name: /^Zoom \d+%$/ }).boundingBox();
  expect(zoom?.y, 'the top of the camera panel').toBeGreaterThanOrEqual(row);
});

test('in flight, holding an item edits the layout, and Return still works while everything jiggles', async ({ vehicle }) => {
  test.setTimeout(120_000);
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  await takeOff(vehicle, '5 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await holdUntilEditing(vehicle, vehicle.getByRole('button', { name: /^Battery \d+%/ }).first());
  const notice = vehicle.getByRole('status').filter({ hasText: 'Editing the layout. Return, Land and Kill motors still work.' });
  await expect(notice, 'in flight the screen says why most of it stopped answering').toBeVisible();
  await vehicle.getByRole('button', { name: 'Return home', exact: true }).click();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to return home' }));
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Return/);
  await expect(editBar(vehicle)).toBeVisible();
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(vehicle)).toBeHidden();
  await expect(notice).toBeHidden();
});

const overlapping = (page: Page, notice: Locator) =>
  notice.evaluate((node) => {
    const box = node.getBoundingClientRect();
    const crosses = (other: DOMRect) => other.width > 0 && other.left < box.right && other.right > box.left && other.top < box.bottom && other.bottom > box.top;
    const items = [...document.querySelectorAll('[data-layout-item], [role=img][aria-label^="Heading "]')].filter((it) => !node.contains(it) && !it.contains(node));
    return items.filter((it) => crosses(it.getBoundingClientRect())).map((it) => it.getAttribute('aria-label') ?? it.querySelector('[aria-label]')?.getAttribute('aria-label') ?? it.textContent?.trim() ?? '?');
  });

test('in flight, the layout editing notice covers neither the heading tape nor any item, and wraps in a narrow window', async ({ vehicle }) => {
  test.setTimeout(120_000);
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  await vehicle.setViewportSize({ width: 1440, height: 900 });
  await takeOff(vehicle, '5 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await editLayout(vehicle);
  const notice = vehicle.getByRole('status').filter({ hasText: 'Editing the layout. Return, Land and Kill motors still work.' });
  await expect(notice).toBeVisible();
  await expect.poll(() => overlapping(vehicle, notice)).toEqual([]);
  await vehicle.setViewportSize({ width: 320, height: 860 });
  await expect(notice).toBeVisible();
  await expect.poll(() => overlapping(vehicle, notice)).toEqual([]);
  const box = await notice.boundingBox();
  expect(box?.x, 'its start is on screen').toBeGreaterThanOrEqual(0);
  expect((box?.x ?? 0) + (box?.width ?? 0), 'its end is on screen').toBeLessThanOrEqual(320);
  expect(box?.height, 'it wraps onto a second line').toBeGreaterThan(40);
});

test('in flight, holding Land lands without starting layout editing', async ({ vehicle }) => {
  test.setTimeout(120_000);
  await takeOff(vehicle, '5 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  const [x, y] = await centre(vehicle.getByRole('button', { name: 'Slide to land' }));
  await vehicle.mouse.move(x, y);
  await vehicle.mouse.down();
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Land/);
  await vehicle.mouse.up();
  await expect(editBar(vehicle)).toBeHidden();
});

test('holding the flight mode chip, which stays where it is while editing, starts layout editing without opening the mode menu', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  await expect(vehicle.getByRole('button', { name: /^(Ready to fly|Not fully ready)$/ }), 'the chip has settled, so nothing moves under the pointer').toBeVisible({ timeout: 60_000 });
  await holdUntilEditing(vehicle, flightMode(vehicle));
  await expect(vehicle.getByRole('menu')).toHaveCount(0);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(vehicle)).toBeHidden();
  await vehicle.getByRole('button', { name: 'Change flight mode', exact: true }).click();
  await expect(vehicle.getByRole('menu'), 'a tap still opens it').toBeVisible();
});

test('holding a reading the picture shows through starts layout editing, and dragging from it does not', async ({ vehicle }) => {
  const reading = vehicle.getByText('H.S', { exact: true }).locator('..');
  await expect(reading).toBeVisible({ timeout: 30_000 });
  const [x, y] = await centre(reading);
  await vehicle.mouse.move(x, y);
  await vehicle.mouse.down();
  await vehicle.mouse.move(x + 60, y, { steps: 6 });
  await pastLongPress(vehicle, await vehicle.evaluate(() => performance.now()));
  await vehicle.mouse.up();
  await expect(editBar(vehicle)).toBeHidden();
  await holdUntilEditing(vehicle, reading);
  await expect(vehicle.getByRole('button', { name: 'Done' })).toBeVisible();
});

test('a three-axis gimbal gets a camera joystick: held down it tilts, held right it turns, let go it stops, and it never starts layout editing', async ({ app }) => {
  test.setTimeout(120_000);
  await withCamera(app);
  const stick = app.getByRole('img', { name: 'Camera joystick' });
  await expect.poll(async () => (await gimbalAim(app)).pitch, { timeout: 60_000 }).not.toBeNull();
  await expect(stick, 'aiming on the picture is the default, so no joystick yet').toHaveCount(0);
  await chooseCameraJoystick(app);
  await expect(stick, 'the mock gimbal reports a yaw axis').toBeVisible({ timeout: 30_000 });
  await expect(app.getByRole('slider', { name: 'Camera tilt' }), 'the joystick replaces the sliders').toHaveCount(0);
  const before = await gimbalAim(app);
  const [x, y] = await centre(stick);
  await app.mouse.move(x, y);
  await app.mouse.down();
  const pressedAt = await app.evaluate(() => performance.now());
  await app.mouse.move(x, y + 20, { steps: 5 });
  await pastLongPress(app, pressedAt);
  await app.mouse.move(x, y + 60, { steps: 5 });
  await expect.poll(async () => (await gimbalAim(app)).pitch!, { timeout: 20_000 }).toBeLessThan(before.pitch! - 10);
  await app.mouse.move(x + 60, y, { steps: 5 });
  await expect.poll(async () => (await gimbalAim(app)).yaw!, { timeout: 20_000 }).toBeGreaterThan((before.yaw ?? 0) + 10);
  await app.mouse.up();
  await expect(editBar(app), 'the drag held the pointer down past a long press, but it moved').toBeHidden();
  const stopped = await gimbalAim(app);
  const releasedAt = await app.evaluate(() => performance.now());
  await app.waitForFunction(([from, wait]) => performance.now() - from > wait, [releasedAt, 1_500] as const);
  expect(Math.abs((await gimbalAim(app)).yaw! - stopped.yaw!), 'let go, it stays where it got to').toBeLessThan(3);
});

test('the camera panel drags to a new place that is still there after a reload', async ({ app }) => {
  await withCamera(app);
  const shutter = app.getByRole('button', { name: 'Start recording' });
  const [x, y] = await centre(shutter);
  await dragCameraPanel(app, -300, 120);
  const [movedX, movedY] = await centre(shutter);
  expect(movedX).toBeLessThan(x - 250);
  expect(movedY).toBeGreaterThan(y + 80);
  const stored = await storedCameraPanel(app);
  expect(stored).toMatch(/^-\d+(\.\d+)?,\d+(\.\d+)?$/);
  await app.reload();
  await withCamera(app);
  await expect.poll(async () => (await centre(shutter))[0]).toBeCloseTo(movedX, -1);
  expect((await centre(shutter))[1]).toBeCloseTo(movedY, -1);
  expect(await storedCameraPanel(app)).toBe(stored);
});

test('the camera panel stays on screen in a smaller window and goes back to its place when the window grows back', async ({ app }) => {
  await withCamera(app);
  const shutter = app.getByRole('button', { name: 'Start recording' });
  await dragCameraPanel(app, 0, 600);
  const stored = await storedCameraPanel(app);
  expect(stored, 'dragged down to the bottom edge').toMatch(/^0,\d+(\.\d+)?$/);
  const placed = await centre(shutter);
  await app.setViewportSize({ width: 900, height: 560 });
  await expect.poll(() => bottomOf(shutter), { message: 'pulled up into the smaller window' }).toBeLessThanOrEqual(560);
  expect(await storedCameraPanel(app), 'the pull-back is not saved').toBe(stored);
  await app.setViewportSize({ width: 1280, height: 800 });
  await expect.poll(async () => (await centre(shutter))[1]).toBeCloseTo(placed[1], -1);
  expect(await storedCameraPanel(app)).toBe(stored);
});

test('each camera control hides on its own and the panel closes up around it', async ({ app }) => {
  await withCamera(app);
  await editLayout(app);
  const shutter = app.getByRole('button', { name: 'Start recording' });
  await shutter.locator('xpath=ancestor::div[contains(@class,"relative")][1]').getByRole('button', { name: 'Hide' }).click();
  await app.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(app)).toBeHidden();
  await expect(shutter, 'the hidden shutter').toBeHidden();
  await expect(app.getByRole('button', { name: /^Zoom \d+%$/ }), 'the zoom stays').toBeVisible();
});

test('the hidden mini map names itself on its badge, and the battery bar hides from the layout options', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  await expect(vehicle.getByRole('img', { name: /^Battery / })).toHaveCount(1);
  await editLayout(vehicle);
  await expect(vehicle.getByRole('button', { name: 'Show mini map' })).toHaveText(/Mini map/);
  await editBar(vehicle).click();
  await vehicle.getByRole('menuitem', { name: 'Battery bar: Shown' }).click();
  await expect(vehicle.getByRole('menuitem', { name: 'Battery bar: Hidden' })).toBeVisible();
  await vehicle.keyboard.press('Escape');
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await expect(vehicle.getByRole('img', { name: /^Battery / })).toHaveCount(0);
});

test('a hide badge takes a tap that lands next to it, not only on its small dot', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  await editLayout(vehicle);
  const badge = vehicle.getByRole('button', { name: 'Show mini map' });
  const box = await badge.boundingBox();
  if (box === null) throw new Error('badge is not on screen');
  await vehicle.mouse.click(box.x + box.width / 2, box.y + box.height + 10);
  await expect(vehicle.getByRole('button', { name: 'Hide mini map' })).toBeVisible();
});

test('in portrait, editing puts the whole status strip on its own line, every indicator on screen and none under Done', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  await vehicle.setViewportSize({ width: 412, height: 860 });
  await editLayout(vehicle);
  const done = await vehicle.getByRole('button', { name: 'Done' }).boundingBox();
  const badges = await vehicle.locator('[data-status-row] [aria-label=Hide], [data-status-row] [aria-label=Show]').evaluateAll((nodes) => nodes.map((node) => node.getBoundingClientRect().toJSON() as DOMRect));
  if (done === null) throw new Error('Done is not on screen');
  expect(badges.length).toBeGreaterThan(2);
  badges.forEach((badge) => {
    expect(badge.left).toBeGreaterThanOrEqual(0);
    expect(badge.right).toBeLessThanOrEqual(412);
    expect(badge.top > done.y + done.height || badge.right < done.x, 'the badge is clear of Done').toBe(true);
  });
});

test('a held status indicator lifts out of its jiggle and settles back when let go', async ({ vehicle }) => {
  await editLayout(vehicle);
  await liftsWhileHeld(vehicle, vehicle.getByRole('button', { name: /^Battery \d+%/ }).first());
});

test('the camera panel lifts as one piece while held and settles back when let go', async ({ app }) => {
  await connectMock(app, ['Enable camera', 'Enable gimbal']);
  const shutter = app.getByRole('button', { name: 'Start recording' });
  await expect(shutter).toBeVisible();
  await editLayout(app);
  await liftsWhileHeld(app, shutter);
});

test('the pitch ladder drags to a new place, while the crosshair stays on the aim point', async ({ vehicle }) => {
  const sight = vehicle.getByRole('group', { name: 'Sight' });
  const pitch = sight.locator('[data-layout-item]').filter({ has: vehicle.getByRole('img', { name: /^Pitch / }) });
  const crosshair = sight.locator('[data-layout-item]').first();
  await expect(pitch).toBeVisible({ timeout: 30_000 });
  await editLayout(vehicle);
  const drag = async (target: Locator, dx: number, dy: number) => {
    const [x, y] = await centre(target);
    await vehicle.mouse.move(x, y);
    await vehicle.mouse.down();
    await vehicle.mouse.move(x + dx, y + dy, { steps: 12 });
    await vehicle.mouse.up();
  };
  const [pitchX, pitchY] = await centre(pitch);
  const [crossX, crossY] = await centre(crosshair);
  await drag(pitch, 160, -60);
  await drag(crosshair, 160, -60);
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(vehicle)).toBeHidden();
  const [movedX, movedY] = await centre(pitch);
  expect(movedX - pitchX).toBeGreaterThan(140);
  expect(pitchY - movedY).toBeGreaterThan(40);
  const [stillX, stillY] = await centre(crosshair);
  expect(Math.hypot(stillX - crossX, stillY - crossY), 'the crosshair marks where the camera points').toBeLessThan(4);
});

test('the heading tape drags to a new place', async ({ vehicle }) => {
  const tape = vehicle.locator('[data-layout-item]').filter({ has: vehicle.getByLabel(/^Heading /) });
  await expect(tape).toBeVisible({ timeout: 30_000 });
  await editLayout(vehicle);
  const [x, y] = await centre(tape);
  await vehicle.mouse.move(x, y);
  await vehicle.mouse.down();
  await vehicle.mouse.move(x - 160, y + 120, { steps: 12 });
  await vehicle.mouse.up();
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(vehicle)).toBeHidden();
  const [movedX, movedY] = await centre(tape);
  expect(x - movedX).toBeGreaterThan(140);
  expect(movedY - y).toBeGreaterThan(100);
});

const shownShare = (target: Locator) =>
  target.evaluate((node) => {
    const box = node.getBoundingClientRect();
    const chain = (at: Element | null): Element[] => (at === null ? [] : [at, ...chain(at.parentElement)]);
    const clip = chain(node.parentElement).reduce(
      (seen, at) => {
        const style = getComputedStyle(at);
        const edge = at.getBoundingClientRect();
        const clipsX = style.overflowX !== 'visible';
        const clipsY = style.overflowY !== 'visible';
        return {
          left: clipsX ? Math.max(seen.left, edge.left) : seen.left,
          right: clipsX ? Math.min(seen.right, edge.right) : seen.right,
          top: clipsY ? Math.max(seen.top, edge.top) : seen.top,
          bottom: clipsY ? Math.min(seen.bottom, edge.bottom) : seen.bottom,
        };
      },
      { left: 0, top: 0, right: innerWidth, bottom: innerHeight },
    );
    const width = Math.max(0, Math.min(box.right, clip.right) - Math.max(box.left, clip.left));
    const height = Math.max(0, Math.min(box.bottom, clip.bottom) - Math.max(box.top, clip.top));
    return Math.round(((width * height) / (box.width * box.height)) * 100) / 100;
  });

test('the motor load widget shows whole while held, dropped lower, hidden, and pushed against the picture edge', async ({ app }) => {
  test.setTimeout(150_000);
  await app.emulateMedia({ reducedMotion: 'reduce' });
  await connectMock(app, [], 'APM ArduCopter Vehicle');
  await takeOff(app, '5 m');
  await expect(flightMode(app)).toHaveAccessibleName(/Flying/, { timeout: 60_000 });
  const motors = app.getByRole('img', { name: 'Motor load' });
  await expect(motors).toBeVisible();
  await editLayout(app);
  const whole = (state: string) => expect.poll(() => shownShare(motors), { message: `${state}, the whole widget shows` }).toBe(1);
  await whole('in place');
  const [x, y] = await centre(motors);
  await app.mouse.move(x, y);
  await app.mouse.down();
  await whole('held');
  await app.mouse.move(x + 120, y + 160, { steps: 12 });
  await whole('carried lower');
  await app.mouse.up();
  await whole('dropped lower');
  await motors.locator('xpath=ancestor::div[contains(@class,"relative")][1]').getByRole('button', { name: 'Hide' }).click();
  await whole('hidden');
  const [hiddenX, hiddenY] = await centre(motors);
  await app.mouse.move(hiddenX, hiddenY);
  await app.mouse.down();
  await app.mouse.move(hiddenX + 3000, hiddenY + 3000, { steps: 12 });
  await whole('held against the corner');
  await app.mouse.up();
  await whole('dropped in the corner');
});

const cuttingBoxes = (page: Page) =>
  page.evaluate(() => {
    const chain = (at: Element | null): Element[] => (at === null ? [] : [at, ...chain(at.parentElement)]);
    const cuts = (at: Element) => {
      const style = getComputedStyle(at);
      const box = at.getBoundingClientRect();
      return (style.overflowX !== 'visible' && box.width < innerWidth - 1) || (style.overflowY !== 'visible' && box.height < innerHeight - 1);
    };
    return [...document.querySelectorAll('[data-layout-item]')]
      .filter((item) => item.parentElement?.querySelector(':scope > .cursor-grab') != null)
      .filter((item) => chain(item.parentElement).some(cuts))
      .map((item) => item.querySelector('[aria-label]')?.getAttribute('aria-label') ?? item.textContent?.trim() ?? '?');
  });

(
  [
    ['landscape', 1280, 800],
    ['portrait', 412, 860],
  ] as const
).forEach(([orientation, width, height]) =>
  test(`in ${orientation} no item that drags sits inside a box smaller than the screen that would cut it off`, async ({ app }) => {
    await app.setViewportSize({ width, height });
    await connectMock(app, ['Enable camera', 'Enable gimbal', 'Enable proximity sensors', 'Send status text + voice']);
    await expect(app.getByRole('button', { name: 'Dismiss messages', exact: true })).toBeVisible();
    await editLayout(app);
    await expect.poll(() => cuttingBoxes(app)).toEqual([]);
  }),
);

const hideBadgeOf = (target: Locator) => target.locator('xpath=ancestor::*[@data-layout-item][1]/..').getByRole('button', { name: 'Hide', exact: true });

test('the gimbal mode switch and the tilt readout on the camera rail are items of their own, each moved and hidden without the other', async ({ app }) => {
  await app.emulateMedia({ reducedMotion: 'reduce' });
  await connectMock(app, ['Enable gimbal']);
  const tilt = app.getByLabel(/^Tilt /);
  await expect(tilt).toBeVisible({ timeout: 60_000 });
  await app.getByRole('button', { name: 'Map', exact: true }).click();
  const mode = app.getByRole('button', { name: /^Video gimbal mode/ });
  await expect(mode, 'with the picture small the mode switch sits on the rail').toBeVisible();
  await editLayout(app);
  await expect(hideBadgeOf(tilt)).toHaveCount(1);
  await expect(hideBadgeOf(mode)).toHaveCount(1);
  const [modeX, modeY] = await centre(mode);
  const [x, y] = await centre(tilt);
  await app.mouse.move(x, y);
  await app.mouse.down();
  await app.mouse.move(x - 200, y + 40, { steps: 12 });
  await app.mouse.up();
  expect((await centre(tilt))[0], 'the tilt readout moved').toBeLessThan(x - 150);
  expect(await centre(mode), 'the mode switch stayed').toEqual([modeX, modeY]);
  await hideBadgeOf(tilt).click();
  await app.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(app)).toBeHidden();
  await expect(tilt).toBeHidden();
  await expect(mode).toBeVisible();
});

const dragBy = async (page: Page, target: Locator, dx: number, dy: number) => {
  const [x, y] = await centre(target);
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + dx, y + dy, { steps: 12 });
  await page.mouse.up();
};

const finishEditing = async (page: Page) => {
  await page.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(page)).toBeHidden();
};

const removeBadgeOf = (target: Locator) => target.locator('xpath=ancestor::*[@data-layout-item][1]/..').getByRole('button', { name: 'Remove', exact: true });

const showBadgeOf = (target: Locator) => target.locator('xpath=ancestor::*[@data-layout-item][1]/..').getByRole('button', { name: 'Show', exact: true });

const topStart = async (target: Locator): Promise<readonly [number, number]> => {
  const box = await target.boundingBox();
  if (box === null) throw new Error('target is not on screen');
  return [Math.round(box.x), Math.round(box.y)];
};

const boxesOf = (targets: readonly Locator[]) => Promise.all(targets.map(async (it) => (await it.boundingBox())!));

const overlaps = (boxes: readonly { x: number; y: number; width: number; height: number }[]) =>
  boxes.flatMap((a, i) => boxes.slice(i + 1).filter((b) => a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height).map(() => i));

test('a reading over the picture drags to a place of its own, keeps it after a reload, and the other readings stay put', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  const distance = telemetry(vehicle, 'D');
  const height = telemetry(vehicle, 'H');
  await expect(distance).toBeVisible({ timeout: 30_000 });
  const [x, y] = await centre(distance);
  const before = await topStart(height);
  await editLayout(vehicle);
  await dragBy(vehicle, distance, 300, 150);
  await finishEditing(vehicle);
  const [movedX, movedY] = await centre(distance);
  expect(movedX - x).toBeGreaterThan(250);
  expect(movedY - y).toBeGreaterThan(120);
  expect(await topStart(height), 'the height reading stayed where it was').toEqual(before);
  await vehicle.reload();
  await connectMock(vehicle);
  await expect.poll(async () => (await centre(distance))[0], { timeout: 30_000 }).toBeCloseTo(movedX, 0);
  expect((await centre(distance))[1]).toBeCloseTo(movedY, 0);
  expect(await topStart(height)).toEqual(before);
});

const pullFromBar = async (page: Page, cell: Locator, dx: number, dy: number) => {
  const [x, y] = await centre(cell);
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.waitForTimeout(300);
  await page.mouse.move(x + dx, y + dy, { steps: 12 });
  await page.mouse.up();
};

test('a reading dropped on the top bar joins its items, and pulled down from the bar it stands over the picture again', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  const distance = telemetry(vehicle, 'D');
  await expect(distance).toBeVisible({ timeout: 30_000 });
  await editLayout(vehicle);
  const strip = vehicle.locator('[data-status-row]');
  const docked = vehicle.locator('[data-indicator="reading:vehicle/distanceToHome"]');
  const flightTime = vehicle.locator('[data-indicator=flightTime]');
  await expect
    .poll(async () => {
      const first = await centre(flightTime);
      await vehicle.waitForTimeout(300);
      return (await centre(flightTime))[0] - first[0];
    }, { message: 'the bar has settled' })
    .toBe(0);
  const [fromX, fromY] = await centre(distance);
  const [timeX, timeY] = await centre(flightTime);
  await dragBy(vehicle, distance, timeX - 30 - fromX, timeY - fromY);
  await expect(docked.getByText('D', { exact: true }), 'the reading is one of the bar items').toHaveCount(1);
  const row = (await strip.boundingBox())!;
  const inBar = (await distance.boundingBox())!;
  expect(inBar.y).toBeGreaterThanOrEqual(row.y - 1);
  expect(inBar.y + inBar.height).toBeLessThanOrEqual(row.y + row.height + 1);
  expect(inBar.x + inBar.width / 2, 'placed where it was dropped, before the flight time').toBeLessThan((await centre(flightTime))[0]);
  expect(await overlaps(await boxesOf([distance, flightTime])), 'the bar made room for it').toEqual([]);
  await finishEditing(vehicle);
  await expect(docked.getByText('D', { exact: true })).toHaveCount(1);
  await vehicle.reload();
  await connectMock(vehicle);
  await expect(docked.getByText('D', { exact: true }), 'kept in the bar after a reload').toHaveCount(1, { timeout: 30_000 });
  await editLayout(vehicle);
  await pullFromBar(vehicle, docked, -200, 260);
  await expect(docked).toHaveCount(0);
  const back = (await distance.boundingBox())!;
  const rowNow = (await strip.boundingBox())!;
  expect(back.y, 'over the picture, below the bar').toBeGreaterThan(rowNow.y + rowNow.height);
  expect(Math.abs(back.y + back.height / 2 - (inBar.y + inBar.height / 2 + 260)), 'near where it was let go').toBeLessThan(60);
  await finishEditing(vehicle);
});

test('a bar item pulled down stands over the picture where it was let go, keeps its place, and dropped on the bar it joins it again', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  await expect(telemetry(vehicle, 'D')).toBeVisible({ timeout: 30_000 });
  await editLayout(vehicle);
  const strip = vehicle.locator('[data-status-row]');
  const inBar = vehicle.locator('[data-indicator=flightTime]');
  const timer = vehicle.getByRole('timer', { name: 'Flight time' });
  await expect(inBar).toHaveCount(1);
  const [barX, barY] = await centre(inBar);
  await pullFromBar(vehicle, inBar, -350, 150);
  await expect(inBar, 'it left the bar').toHaveCount(0);
  await expect(timer).toBeVisible();
  const afloat = await centre(timer);
  expect(Math.abs(afloat[0] - (barX - 350))).toBeLessThan(60);
  expect(Math.abs(afloat[1] - (barY + 150))).toBeLessThan(60);
  await finishEditing(vehicle);
  await vehicle.reload();
  await connectMock(vehicle);
  await expect(timer).toBeVisible({ timeout: 30_000 });
  expect(Math.abs((await centre(timer))[1] - afloat[1]), 'kept over the picture after a reload').toBeLessThan(8);
  await expect(inBar).toHaveCount(0);
  await editLayout(vehicle);
  const row = (await strip.boundingBox())!;
  const [x, y] = await centre(timer);
  await vehicle.mouse.move(x, y);
  await vehicle.mouse.down();
  await vehicle.mouse.move(barX, row.y + row.height / 2, { steps: 12 });
  expect(Math.abs((await centre(timer))[1] - (row.y + row.height / 2)), 'it follows the pointer into the bar').toBeLessThan(12);
  await vehicle.mouse.up();
  await expect(inBar, 'back among the bar items').toHaveCount(1);
  const home = (await inBar.boundingBox())!;
  expect(home.y).toBeGreaterThanOrEqual(row.y - 1);
  expect(home.y + home.height).toBeLessThanOrEqual(row.y + row.height + 1);
  await finishEditing(vehicle);
});

test('a reading leaves with its own badge and no dimmed copy stays behind, and the arrow keys move one', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  const speed = telemetry(vehicle, 'H.S');
  const height = telemetry(vehicle, 'H');
  await expect(speed).toBeVisible({ timeout: 30_000 });
  const [x, y] = await centre(speed);
  await editLayout(vehicle);
  await removeBadgeOf(height).click();
  await expect(height, 'gone while the layout is still being edited').toHaveCount(0);
  const move = speed.locator('xpath=ancestor::*[@data-layout-item][1]/..').getByRole('button', { name: 'Move', exact: true });
  await move.focus();
  await vehicle.keyboard.press('ArrowLeft');
  await vehicle.keyboard.press('Shift+ArrowLeft');
  await vehicle.keyboard.press('ArrowDown');
  await finishEditing(vehicle);
  await expect(height).toHaveCount(0);
  await editLayout(vehicle);
  await vehicle.getByRole('button', { name: 'Add reading' }).click();
  const chooser = vehicle.getByRole('dialog', { name: 'Choose readings' });
  await expect(chooser.getByRole('checkbox', { checked: true }), 'the removed reading is no longer chosen').toHaveCount(3);
  await vehicle.keyboard.press('Escape');
  await finishEditing(vehicle);
  await expect(speed).toBeVisible();
  const [movedX, movedY] = await centre(speed);
  expect(Math.round(x - movedX)).toBe(40);
  expect(Math.round(movedY - y)).toBe(8);
});

test('from Add reading on the editing bar a reading is added where nothing else stands and a removed one leaves the layout', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  const readings = ['D', 'H', 'H.S', 'V.S'].map((label) => telemetry(vehicle, label));
  await expect(readings[0]!).toBeVisible({ timeout: 30_000 });
  await editLayout(vehicle);
  await dragBy(vehicle, readings[1]!, 0, 30);
  await vehicle.getByRole('button', { name: 'Add reading' }).click();
  const chooser = vehicle.getByRole('dialog', { name: 'Choose readings' });
  await chooser.getByText('Heading', { exact: true }).first().click();
  await chooser.getByText('Ground speed', { exact: true }).click();
  await vehicle.keyboard.press('Escape');
  await expect(chooser).toBeHidden();
  await expect(editBar(vehicle), 'still editing the layout').toBeVisible();
  const heading = vehicle.getByText('HDG', { exact: true }).locator('..');
  await expect(heading).toBeVisible();
  await expect(readings[2]!, 'the ground speed reading left with its choice').toHaveCount(0);
  await expect(removeBadgeOf(heading)).toHaveCount(1);
  await expect.poll(async () => overlaps(await boxesOf([heading, readings[0]!, readings[1]!, readings[3]!])), { message: 'no reading on another' }).toEqual([]);
});

test('an added reading shows as a glyph and its value, with a short tag where readings share the glyph, never as a spelled-out label', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  await expect(telemetry(vehicle, 'D')).toBeVisible({ timeout: 30_000 });
  await editLayout(vehicle);
  await vehicle.getByRole('button', { name: 'Add reading' }).click();
  const chooser = vehicle.getByRole('dialog', { name: 'Choose readings' });
  await chooser.getByText('Vibration X', { exact: true }).click();
  await chooser.getByText('Voltage', { exact: true }).first().click();
  await vehicle.keyboard.press('Escape');
  await expect(chooser).toBeHidden();
  const shake = vehicle.locator('[role=img][aria-label=vibration]').locator('xpath=ancestor::*[@data-layout-item][1]');
  await expect(shake).toBeVisible();
  await expect(shake.getByText('X', { exact: true }), 'the axis it reads').toBeVisible();
  await expect(vehicle.locator('[data-layout-item] [role=img][aria-label=bolt]'), 'voltage has a glyph of its own').toBeVisible();
  const spelled = await vehicle.evaluate(() => [...document.querySelectorAll('[data-layout-item] *')].filter((node) => node.children.length === 0 && node.closest('.sr-only') === null).map((node) => node.textContent?.trim() ?? '').filter((shown) => /^(VIBRATION X|VOLTAGE)$/.test(shown)));
  expect(spelled, 'no reading spells its name out').toEqual([]);
  await finishEditing(vehicle);
});

test('a search in Choose readings narrows the list to what was typed, by reading or by group, and says when nothing matches', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  await expect(telemetry(vehicle, 'D')).toBeVisible({ timeout: 30_000 });
  await editLayout(vehicle);
  await vehicle.getByRole('button', { name: 'Add reading' }).click();
  const chooser = vehicle.getByRole('dialog', { name: 'Choose readings' });
  const search = chooser.getByRole('textbox', { name: 'Search readings' });
  await expect(chooser.getByText('Roll', { exact: true })).toBeVisible();
  await chooser.getByText('Vibration Z', { exact: true }).scrollIntoViewIfNeeded();
  await expect(search, 'the search stays in reach however far the list is scrolled').toBeInViewport({ ratio: 1 });
  await search.fill('vibration x');
  await expect(chooser.getByText('Vibration X', { exact: true })).toBeVisible();
  await expect(chooser.getByText('Roll', { exact: true })).toHaveCount(0);
  await expect(chooser.getByText('Vibration Y', { exact: true })).toHaveCount(0);
  await chooser.getByText('Vibration X', { exact: true }).click();
  await search.fill('no such reading');
  await expect(chooser.getByText('No reading matches "no such reading".')).toBeVisible();
  await search.fill('');
  await expect(chooser.getByText('Roll', { exact: true })).toBeVisible();
  await vehicle.keyboard.press('Escape');
  await expect(chooser).toBeHidden();
  await expect(vehicle.locator('[data-layout-item] [role=img][aria-label=vibration]'), 'the reading found by search was added').toBeVisible();
});

test('a reading is made bigger from its own badge and stays so after a reload, and reset in the layout options puts moved and resized readings back', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  const distance = telemetry(vehicle, 'D');
  const climb = telemetry(vehicle, 'V.S');
  await expect(distance).toBeVisible({ timeout: 30_000 });
  const placed = await topStart(distance);
  const tall = async () => (await climb.boundingBox())!.height;
  const normal = await tall();
  await editLayout(vehicle);
  await dragBy(vehicle, distance, 200, 100);
  const item = climb.locator('xpath=ancestor::*[@data-layout-item][1]/..');
  await item.getByRole('button', { name: 'Make bigger', exact: true }).click();
  await item.getByRole('button', { name: 'Make bigger', exact: true }).click();
  await expect.poll(tall).toBeCloseTo(normal * 1.5, 0);
  await finishEditing(vehicle);
  await vehicle.reload();
  await connectMock(vehicle);
  await expect(climb).toBeVisible({ timeout: 30_000 });
  await expect.poll(tall, { message: 'kept after a reload' }).toBeCloseTo(normal * 1.5, 0);
  await editLayout(vehicle);
  await editBar(vehicle).click();
  await vehicle.getByRole('menuitem', { name: 'Reset positions and hidden items' }).click();
  await vehicle.getByRole('menuitem', { name: /again to reset$/ }).click();
  await finishEditing(vehicle);
  await expect.poll(tall).toBeCloseTo(normal, 0);
  expect(await topStart(distance)).toEqual(placed);
});

test('on a phone the readings over the picture move on their own and stop at the edges of the picture, clear of the deck and the status strip', async ({ app }) => {
  await app.emulateMedia({ reducedMotion: 'reduce' });
  await app.setViewportSize({ width: 412, height: 860 });
  await connectMock(app, ['Enable camera', 'Enable gimbal']);
  await expect(app.getByRole('button', { name: /^Cameras, showing/ }), 'the picture has opened above the map').toBeVisible({ timeout: 30_000 });
  const speed = telemetry(app, 'H.S');
  const climb = telemetry(app, 'V.S');
  await expect(speed).toBeVisible({ timeout: 30_000 });
  const distance = telemetry(app, 'D');
  const map = app.getByRole('region', { name: 'Map' });
  const apart = async () => [(await centre(climb))[0] - (await centre(distance))[0], (await centre(climb))[1] - (await centre(distance))[1]];
  const still = await apart();
  await editLayout(app);
  await dragBy(app, speed, 60, 900);
  const pictureBottom = (await map.boundingBox())!.y;
  expect((await speed.boundingBox())!.y + (await speed.boundingBox())!.height, 'held above the map and the deck below it').toBeLessThanOrEqual(pictureBottom + 1);
  await dragBy(app, speed, 0, -900);
  const strip = (await app.locator('[data-status-row]').boundingBox())!;
  expect((await speed.boundingBox())!.y, 'held below the status strip').toBeGreaterThanOrEqual(strip.y + strip.height - 1);
  const picker = app.getByRole('button', { name: /^Cameras, showing/ });
  await dragBy(app, picker, 0, 900);
  expect((await picker.boundingBox())!.y + (await picker.boundingBox())!.height, 'the camera picker stays on the picture too').toBeLessThanOrEqual(pictureBottom + 1);
  await finishEditing(app);
  const [climbX, climbY] = await apart();
  expect(climbY, 'the climb rate stayed on its row beside the distance').toBeCloseTo(still[1]!, 0);
  expect(Math.abs(climbX - still[0]!), 'and in its place along it, give or take a digit').toBeLessThan(12);
});

const withPicture = async (page: Page) => {
  await connectMock(page, ['Enable gimbal']);
  await expect.poll(() => page.locator('video').first().evaluate((element: HTMLVideoElement) => element.videoWidth), { timeout: 30_000 }).toBe(640);
  await flyNow(page);
};

test('the gimbal mode switch on the picture and the camera picker each move and hide on their own, keep their places and stop at the edge of the picture', async ({ app }) => {
  test.setTimeout(120_000);
  await app.emulateMedia({ reducedMotion: 'reduce' });
  await addVideoSource(app, /^WebRTC \(WHEP\)/, WHEP_STREAM);
  await withPicture(app);
  const mode = app.getByRole('button', { name: /^Video gimbal mode/ });
  const picker = app.getByRole('button', { name: /^Cameras, showing/ });
  await expect(mode).toBeVisible({ timeout: 30_000 });
  await expect(picker, 'the gimbal offers its settings from the picker').toBeVisible({ timeout: 30_000 });
  await editLayout(app);
  await expect(hideBadgeOf(mode)).toHaveCount(1);
  await expect(hideBadgeOf(picker)).toHaveCount(1);
  const pickerAt = await centre(picker);
  const [modeX, modeY] = await centre(mode);
  await dragBy(app, mode, -400, -200);
  const [movedX, movedY] = await centre(mode);
  expect(modeX - movedX).toBeGreaterThan(350);
  expect(modeY - movedY).toBeGreaterThan(150);
  expect(await centre(picker), 'the picker stayed').toEqual(pickerAt);
  await dragBy(app, picker, 3000, 3000);
  await expect.poll(() => shownShare(picker), { message: 'pushed into the corner, the picker shows whole' }).toBe(1);
  await hideBadgeOf(mode).click();
  await finishEditing(app);
  await expect(mode, 'hidden on the picture, and not moved onto the rail').toBeHidden();
  await expect(picker).toBeVisible();
  const placed = await centre(picker);
  await app.reload();
  await withPicture(app);
  await expect.poll(async () => (await centre(picker))[0], { timeout: 30_000 }).toBeCloseTo(placed[0], 0);
  expect((await centre(picker))[1]).toBeCloseTo(placed[1], 0);
  await expect(mode).toBeHidden();
});


test('a click on empty space ends layout editing, while a click on an item or with the options menu open keeps it', async ({ vehicle }) => {
  await editLayout(vehicle);
  const [x, y] = await centre(vehicle.getByRole('timer', { name: 'Flight time' }));
  await vehicle.mouse.click(x, y);
  await expect(editBar(vehicle), 'a click on an item keeps editing').toBeVisible();
  await editBar(vehicle).click();
  await expect(vehicle.getByRole('menu')).toBeVisible();
  const view = vehicle.viewportSize()!;
  await vehicle.mouse.click(view.width * 0.35, view.height * 0.72);
  await expect(vehicle.getByRole('menu')).toBeHidden();
  await expect(editBar(vehicle), 'the first click outside only closed the menu').toBeVisible();
  await vehicle.mouse.click(view.width * 0.35, view.height * 0.72);
  await expect(editBar(vehicle), 'a click on empty space ends editing, like tapping the iPhone home screen').toBeHidden();
  await expect(vehicle.getByRole('button', { name: 'Menu', exact: true })).toBeVisible();
});

const holdAndDrag = async (page: Page, target: Locator, dx: number, dy: number) => {
  const [x, y] = await centre(target);
  await page.mouse.move(x, y);
  await page.mouse.down();
  await expect(editBar(page), 'the hold started layout editing').toBeVisible();
  await expect.poll(() => aloft(target), { message: 'and lifted the held item' }).toMatchObject({ scale: 1.1 });
  await page.mouse.move(x + dx, y + dy, { steps: 12 });
  await page.mouse.up();
};

test('a reading held outside layout editing lifts, and the same press drags it on its own', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  const height = telemetry(vehicle, 'H');
  const distance = telemetry(vehicle, 'D');
  await expect(height).toBeVisible({ timeout: 30_000 });
  const [x, y] = await centre(height);
  const still = await topStart(distance);
  await holdAndDrag(vehicle, height, 250, 200);
  await expect(editBar(vehicle), 'still editing after the drag').toBeVisible();
  await finishEditing(vehicle);
  const [movedX, movedY] = await centre(height);
  expect(movedX - x).toBeGreaterThan(200);
  expect(movedY - y).toBeGreaterThan(150);
  expect(await topStart(distance), 'the distance reading stayed').toEqual(still);
});

test('the camera panel held by its shutter outside layout editing drags in the same press', async ({ app }) => {
  await withCamera(app);
  const shutter = app.getByRole('button', { name: 'Start recording' });
  const [x, y] = await centre(shutter);
  await holdAndDrag(app, shutter, -300, 120);
  await finishEditing(app);
  await expect(app.getByRole('button', { name: 'Stop recording' }), 'the hold and drag did not record').toHaveCount(0);
  const [movedX, movedY] = await centre(shutter);
  expect(x - movedX).toBeGreaterThan(250);
  expect(movedY - y).toBeGreaterThan(80);
});

const moveHandleOf = (target: Locator) => target.locator('xpath=ancestor::*[@data-layout-item][1]/..').getByRole('button', { name: 'Move', exact: true });

test('on the ground each deck action drags on its own, has no hide badge, and Reset puts it back', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  const takeOff = vehicle.getByRole('group', { name: 'Flight actions' }).getByRole('button', { name: 'Take off' });
  await expect(takeOff).toBeVisible();
  const [x, y] = await centre(takeOff);
  await editLayout(vehicle);
  await expect(hideBadgeOf(takeOff)).toHaveCount(0);
  await dragBy(vehicle, takeOff, 300, -100);
  const [movedX, movedY] = await centre(takeOff);
  expect(movedX - x).toBeGreaterThan(250);
  expect(y - movedY).toBeGreaterThan(80);
  await editBar(vehicle).click();
  await vehicle.getByRole('menuitem', { name: 'Reset positions and hidden items' }).click();
  await vehicle.getByRole('menuitem', { name: /again to reset$/ }).click();
  await finishEditing(vehicle);
  await expect.poll(() => centre(takeOff)).toEqual([x, y]);
});

test('in flight each deck action moves by its own handle, none hides, and a tap still asks its confirm', async ({ vehicle }) => {
  test.setTimeout(120_000);
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  await takeOff(vehicle, '5 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  const deck = vehicle.getByRole('group', { name: 'Flight actions' });
  const actions = ['Return home', 'Slide to land', 'Kill motors'].map((name) => deck.getByRole('button', { name, exact: true }));
  await expect(actions[0]!).toBeVisible();
  const land = await centre(actions[1]!);
  const [x, y] = await centre(actions[0]!);
  await editLayout(vehicle);
  await Promise.all(actions.map(async (action) => {
    await expect(moveHandleOf(action)).toHaveCount(1);
    await expect(hideBadgeOf(action)).toHaveCount(0);
  }));
  await dragBy(vehicle, moveHandleOf(actions[0]!), 260, 40);
  expect((await centre(actions[0]!))[0] - x, 'return home moved').toBeGreaterThan(220);
  expect(await centre(actions[1]!), 'land stayed').toEqual(land);
  await actions[0]!.click();
  await expect(vehicle.getByRole('button', { name: 'Slide to return home' }), 'a tap on the moved action still asks its confirm').toBeVisible();
  await vehicle.getByRole('button', { name: 'Cancel' }).click();
  await finishEditing(vehicle);
  expect((await centre(actions[0]!))[1]).toBeCloseTo(y + 40, -1);
});

test('on a phone the deck buttons drag on their own and stop below the status strip', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  await vehicle.setViewportSize({ width: 412, height: 860 });
  const takeOff = vehicle.getByRole('group', { name: 'Flight actions' }).getByRole('button', { name: 'Take off' });
  await expect(takeOff).toBeVisible();
  await editLayout(vehicle);
  await expect(hideBadgeOf(takeOff)).toHaveCount(0);
  await dragBy(vehicle, takeOff, 0, -2000);
  const strip = (await vehicle.locator('[data-status-row]').boundingBox())!;
  expect((await takeOff.boundingBox())!.y, 'held below the status strip').toBeGreaterThanOrEqual(strip.y + strip.height - 1);
  await finishEditing(vehicle);
  await expect.poll(() => shownShare(takeOff), { message: 'shown whole after the drop' }).toBe(1);
});

test('over the map the chosen readings, the payload strip and the map buttons each move in Edit layout, and a reading keeps the place it has over the picture', async ({ vehicle }) => {
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  const height = telemetry(vehicle, 'H');
  await expect(height).toBeVisible({ timeout: 30_000 });
  await showMapFull(vehicle);
  const layers = vehicle.getByRole('button', { name: 'Map layers' });
  const payload = vehicle.getByRole('group', { name: 'Payload' });
  await expect(layers).toBeVisible();
  await expect(payload).toBeVisible();
  await expect(telemetry(vehicle, 'D'), 'one reading each, not a block').toBeVisible();
  await editLayout(vehicle);
  await expect(removeBadgeOf(height)).toHaveCount(1);
  const moved = async (target: Locator, dx: number, dy: number) => {
    const [x, y] = await centre(target);
    await dragBy(vehicle, target, dx, dy);
    const [toX, toY] = await centre(target);
    expect(Math.abs(toX - x - dx), 'it went across').toBeLessThan(Math.abs(dx) * 0.2 + 2);
    expect(Math.abs(toY - y - dy), 'it went up or down').toBeLessThan(Math.abs(dy) * 0.2 + 2);
  };
  await moved(height, 250, 150);
  await moved(layers, -300, -100);
  await moved(payload, 0, -200);
  await finishEditing(vehicle);
  const overMap = await topStart(height);
  await vehicle.getByRole('button', { name: 'Video', exact: true }).click();
  await expect(vehicle.getByRole('img', { name: /^Pitch / })).toBeVisible();
  await expect.poll(() => topStart(height), { message: 'the same reading in the same place over the picture' }).toEqual(overMap);
});

test('the heading tape hides from layout editing', async ({ vehicle }) => {
  const tape = vehicle.getByRole('img', { name: /^Heading / });
  await expect(tape).toBeVisible();
  await vehicle.emulateMedia({ reducedMotion: 'reduce' });
  await editLayout(vehicle);
  await tape.locator('xpath=ancestor::div[contains(@class,"relative")][1]').getByRole('button', { name: 'Hide' }).click();
  await vehicle.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(vehicle)).toBeHidden();
  await expect(tape).toBeHidden();
});

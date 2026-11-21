import type { Locator, Page } from '@playwright/test';
import { chooseCameraJoystick, connectMock, expect, flightMode, slideAcross, takeOff, test } from './fixtures';

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

const EDIT_CONTROLS = ['Layout options', 'Done'];

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

test('while editing the layout the video takes no input, and it does again after Done', async ({ app }) => {
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
  await app.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(app)).toBeHidden();
  expect(await presses(), 'the press made while editing').toBe(0);
  await app.mouse.click(x, y);
  await expect.poll(presses, { message: 'the probe point is the video' }).toBe(1);
});

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
  await notice.getByRole('button', { name: 'Done' }).click();
  await expect(editBar(vehicle)).toBeHidden();
  await expect(notice).toBeHidden();
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

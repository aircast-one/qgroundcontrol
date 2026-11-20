import type { Page } from '@playwright/test';
import { connectMock, expect, flightMode, mapMenuAt, openMenu, showMapFull, slideAcross, takeOff, telemetry, test } from './fixtures';
import { type Sitl, connectSitl, startSitl } from './sitl';

test.describe.configure({ timeout: 120_000 });

test('takeoff climbs to the chosen height and holds there', async ({ vehicle }) => {
  await takeOff(vehicle, '10 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^(Takeoff|Hold) Flying/);
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await expect(telemetry(vehicle, 'H')).toHaveText(/^H(9\.[5-9]|10\.[0-4]) m$/);
});

test('slide to land brings the aircraft down and offers takeoff again', async ({ vehicle }) => {
  await takeOff(vehicle, '5 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to land' }));
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Land/);
  await expect(vehicle.getByRole('button', { name: 'Take off' })).toBeVisible({ timeout: 90_000 });
});

test('return switches the aircraft into return mode', async ({ vehicle }) => {
  await takeOff(vehicle, '5 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await vehicle.getByRole('button', { name: 'Return', exact: true }).click();
  await expect(vehicle.getByText('HOME', { exact: true }), 'the confirm says how far home is').toBeVisible();
  await expect(vehicle.getByText(/^\d+ (m|ft)$/).first()).toBeVisible();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to return' }));
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Return/);
});

test('the flight mode menu changes the mode', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'Change flight mode', exact: true }).click();
  const menu = vehicle.getByRole('menu');
  await expect(menu).toBeVisible();
  await menu.getByRole('menuitem', { name: 'Position' }).click();
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Position/);
});

test('a red reading that makes the chip say not fully ready is the take-off warning too, and take-off still goes', async ({ vehicle }) => {
  await expect(vehicle.getByRole('button', { name: 'Not fully ready', exact: true })).toBeVisible();
  await vehicle.getByRole('button', { name: 'Take off' }).click();
  await expect(vehicle.getByText(/^Not fully ready\. Vibration \d+\.\d m\/s²\./)).toBeVisible();
  await vehicle.getByRole('button', { name: '5 m', exact: true }).click();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: /^Slide to take off anyway · 5(\.0)? m$/ }));
  await expect(flightMode(vehicle)).toHaveAccessibleName(/Flying/, { timeout: 60_000 });
});

const parameterValue = (page: Page, name: string) =>
  page.evaluate(
    async ([module, asked]) => {
      const { Qgc } = (await import(module!)) as { Qgc: { get: (path: string) => Promise<{ rawValue?: number } | null> } };
      return (await Qgc.get(`vehicle.parameterManager.getParameter(-1,${asked})`))?.rawValue ?? null;
    },
    ['/src/bridge/Qgc.ts', name],
  );

test('an ArduPlane picks its takeoff height in the take-off panel, writes it before taking off, and the mode menu lists Takeoff once', async ({ app }) => {
  await connectMock(app, [], 'APM ArduPlane Vehicle');
  await expect.poll(() => parameterValue(app, 'TKOFF_ALT'), { message: 'TKOFF_ALT has arrived with the parameters', timeout: 60_000 }).toBe(50);
  const panel = app.getByText('Height above launch');
  await expect(async () => {
    if (await app.getByRole('button', { name: 'Cancel' }).isVisible()) await app.getByRole('button', { name: 'Cancel' }).click();
    await app.getByRole('button', { name: 'Take off' }).click();
    await expect(panel, 'a wing picks its height like a copter once TKOFF_ALT is known').toBeVisible({ timeout: 2_000 });
  }).toPass({ timeout: 30_000 });
  await expect(app.getByText(/^50(\.0)? m$/).first(), 'it opens at the takeoff height the aircraft already has').toBeVisible();
  await app.getByRole('button', { name: '20 m', exact: true }).click();
  await expect(app.getByText('The aircraft throttles up, climbs to 20.0 m above launch and circles there.'), 'a wing does not hold position').toBeVisible();
  await expect(app.getByText(/^Not fully ready\./), 'taking off warns the same way on every frame').toBeVisible();
  await slideAcross(app, app.getByRole('button', { name: /^Slide to take off anyway · 20(\.0)? m$/ }));
  await expect.poll(() => parameterValue(app, 'TKOFF_ALT'), { message: 'the chosen height is written to TKOFF_ALT', timeout: 30_000 }).toBe(20);
  await expect(flightMode(app)).toHaveAccessibleName(/^Takeoff/, { timeout: 30_000 });
  await expect(telemetry(app, 'H')).toHaveText(/^H(1[5-9]|2[0-4])\.\d m$/, { timeout: 90_000 });
  await app.getByRole('button', { name: 'Change flight mode', exact: true }).click();
  await expect(app.getByRole('menu').getByRole('menuitem', { name: /^Take ?off/ }), 'taking off already, the menu offers no second Take off row').toHaveCount(1);
});

const refuseParameter = (page: Page, name: string) =>
  page.evaluate(
    async ([module, refused]) => {
      const { Qgc } = (await import(module!)) as { Qgc: { invoke: (path: string, ...args: unknown[]) => Promise<unknown> } };
      return Qgc.invoke('links.changeMockLinks', { refuseParam: refused });
    },
    ['/src/bridge/Qgc.ts', name],
  );

test('an ArduPlane whose autopilot refuses the new takeoff height says so and does not take off', async ({ app }) => {
  await connectMock(app, [], 'APM ArduPlane Vehicle');
  await expect.poll(() => parameterValue(app, 'TKOFF_ALT'), { timeout: 60_000 }).toBe(50);
  expect(await refuseParameter(app, 'TKOFF_ALT'), 'the simulated plane now keeps TKOFF_ALT whatever it is sent').toBe(true);
  await expect(async () => {
    if (await app.getByRole('button', { name: 'Cancel' }).isVisible()) await app.getByRole('button', { name: 'Cancel' }).click();
    await app.getByRole('button', { name: 'Take off' }).click();
    await expect(app.getByText('Height above launch')).toBeVisible({ timeout: 2_000 });
  }).toPass({ timeout: 30_000 });
  await app.getByRole('button', { name: '20 m', exact: true }).click();
  await slideAcross(app, app.getByRole('button', { name: /^Slide to take off anyway · 20(\.0)? m$/ }));
  await expect(app.getByText('The aircraft did not take the 20.0 m takeoff height, so it did not take off.').first()).toBeVisible({ timeout: 30_000 });
  await expect(app.getByText(/^Parameter write failed/), 'one message says what happened, not a second one about the parameter').toHaveCount(0);
  expect(await parameterValue(app, 'TKOFF_ALT'), 'the aircraft kept its own height').toBe(50);
  await app.getByRole('dialog').filter({ hasText: 'did not take the 20.0 m takeoff height' }).getByRole('button', { name: 'OK' }).click();
  await expect(flightMode(app), 'it never switched to Takeoff or armed').toHaveAccessibleName(/^Manual Not fully ready/);
});

test('Escape cancels the takeoff panel', async ({ vehicle }) => {
  await vehicle.getByRole('button', { name: 'Take off' }).click();
  await expect(vehicle.getByText('Height above launch')).toBeVisible();
  await vehicle.keyboard.press('Escape');
  await expect(vehicle.getByText('Height above launch')).toBeHidden();
});

const QGC = '/src/bridge/Qgc.ts';

const dropLink = (page: Page) =>
  page.evaluate(async (module) => {
    const { Qgc } = (await import(module)) as { Qgc: { invoke: (path: string) => Promise<boolean> } };
    await Qgc.invoke('vehicle.closeVehicle');
  }, QGC);

const coreAnswered = (page: Page, times: number) =>
  page.evaluate(
    async ({ module, times }) => {
      const { Qgc } = (await import(module)) as { Qgc: { get: (path: string) => Promise<unknown> } };
      await Array.from({ length: times }).reduce<Promise<unknown>>((done) => done.then(() => Qgc.get('view.flyState')), Promise.resolve());
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    },
    { module: QGC, times },
  );

type Heard = { heard: readonly string[] };

const recordNotices = (page: Page) =>
  page.evaluate(() => {
    const record = window as unknown as Heard;
    record.heard = [];
    new MutationObserver(() => {
      record.heard = [...record.heard, ...[...document.querySelectorAll('[role="status"]')].map((it) => it.textContent ?? '')];
    }).observe(document.body, { subtree: true, childList: true, characterData: true });
  });

const listenForSpeech = (page: Page) =>
  page.evaluate(() => {
    const record = window as unknown as Heard;
    record.heard = [];
    const play = HTMLMediaElement.prototype.play;
    HTMLMediaElement.prototype.play = function played(this: HTMLMediaElement) {
      record.heard = [...record.heard, this.src];
      return play.call(this);
    };
  });

const heard = (page: Page): Promise<readonly string[]> => page.evaluate(() => (window as unknown as Heard).heard);

type MapSources = { sources: Readonly<Record<string, string>> };

const recordMapSources = (page: Page) =>
  page.evaluate(async () => {
    const record = window as unknown as MapSources;
    record.sources = {};
    const url = (await (await fetch('/src/map/VehicleMap.tsx')).text()).match(/"([^"]*\/maplibre-gl\.js[^"]*)"/)?.[1];
    if (url === undefined) throw new Error('the map module does not import the map library');
    const { GeoJSONSource } = (await import(url)) as { GeoJSONSource: { prototype: { id: string; setData: (data: unknown) => unknown } } };
    const setData = GeoJSONSource.prototype.setData;
    GeoJSONSource.prototype.setData = function recorded(this: { id: string }, data: unknown) {
      record.sources = { ...record.sources, [this.id]: JSON.stringify(data) };
      return setData.call(this, data);
    };
  });

const mapSource = (page: Page, id: string): Promise<string> => page.evaluate((id) => (window as unknown as MapSources).sources[id] ?? '', id);

test('a link that drops mid-flight says the link is lost and where the aircraft was, never that it landed', async ({ vehicle }) => {
  await takeOff(vehicle, '5 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/Flying/, { timeout: 60_000 });
  await recordNotices(vehicle);
  await dropLink(vehicle);
  const alert = vehicle.getByRole('alert').filter({ hasText: 'Link lost' });
  await expect(alert).toContainText(/Last seen at -?\d+\.\d{5}, -?\d+\.\d{5}/);
  await expect(alert).toContainText(/\d+(\.\d+)? m up/);
  await expect(alert).toContainText('Failsafe on link loss:');
  await expect(vehicle.getByText('Link lost', { exact: true })).toBeVisible();
  await coreAnswered(vehicle, 3);
  expect((await heard(vehicle)).filter((it) => /disarmed/i.test(it)), 'no landed or disarmed notice, even briefly').toEqual([]);
  await expect(vehicle.getByText(/disarmed/i)).toHaveCount(0);
  await expect(vehicle.getByText(/^Turn on the aircraft/)).toHaveCount(0);
});

const silenceLink = (page: Page, silent: boolean) =>
  page.evaluate(
    async ({ module, silent }) => {
      const { Qgc } = (await import(module)) as { Qgc: { invoke: (path: string, ...args: unknown[]) => Promise<unknown> } };
      return Qgc.invoke('links.changeMockLinks', { silent: silent ? '1' : '0' });
    },
    { module: QGC, silent },
  );

const listenForWords = (page: Page) =>
  page.evaluate(async (module) => {
    const { SpeechOut } = (await import(module)) as { SpeechOut: { say: (text: string, clip: string) => void } };
    const say = SpeechOut.say;
    SpeechOut.say = (text: string, clip: string) => {
      const record = window as unknown as Heard;
      record.heard = [...record.heard, text];
      say(text, clip);
    };
  }, '/src/app/SpeechOut.ts');

test('a link that goes quiet in the air says the link is lost rather than naming the motors, marks the readings stale and offers no disarm', async ({ app: vehicle }) => {
  await connectMock(vehicle, [], 'APM ArduCopter Vehicle');
  await takeOff(vehicle, '5 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/Flying/, { timeout: 60_000 });
  await expect(vehicle.getByRole('img', { name: 'Motor load' }), 'the motors report their telemetry before the link goes').toBeVisible();
  await listenForSpeech(vehicle);
  await listenForWords(vehicle);
  expect(await silenceLink(vehicle, true), 'the simulated link goes quiet both ways, like a radio out of range').toBeTruthy();
  await expect.poll(() => heard(vehicle), { timeout: 20_000 }).toContainEqual(expect.stringMatching(/signal_lost/));
  expect((await heard(vehicle)).filter((it) => /telemetry lost/i.test(it)), 'every motor going quiet with the link is the link, not the motors').toEqual([]);
  const stale = vehicle.getByText('No contact — these are the last values the vehicle sent.');
  await expect(stale, 'the readings over the picture say they are the last ones heard').toBeVisible();
  await vehicle.getByRole('button', { name: /^\d+ s · / }).first().click();
  const sheet = vehicle.getByRole('dialog', { name: 'Vehicle status' });
  await expect(sheet.getByText('If the aircraft loses this app')).toBeVisible();
  await expect(sheet.getByText('Slide to disarm'), 'with contact lost the aircraft may still be in the air').toHaveCount(0);
  await vehicle.keyboard.press('Escape');
  await silenceLink(vehicle, false);
  await expect(stale).toBeHidden({ timeout: 20_000 });
});

const dropsOn = (screen: 'Settings' | 'Plan') =>
  test(`a link that drops while ${screen} is open raises the link-lost bar and says so aloud`, async ({ vehicle }) => {
    await takeOff(vehicle, '5 m');
    await expect(flightMode(vehicle)).toHaveAccessibleName(/Flying/, { timeout: 60_000 });
    await openMenu(vehicle, screen);
    await listenForSpeech(vehicle);
    await dropLink(vehicle);
    const bar = vehicle.getByRole('alert', { name: 'Link lost' });
    await expect(bar).toContainText(/^Link lost · /);
    await expect(bar.getByLabel(/^Last alt \d+(\.\d+)? m$/)).toBeVisible();
    await expect(bar.getByLabel(/^Last battery \d+%$/)).toBeVisible();
    await expect.poll(() => heard(vehicle)).toContainEqual(expect.stringMatching(/signal_lost/));
    await bar.getByRole('button', { name: 'Back to Fly' }).click();
    await expect(bar).toBeHidden();
    await expect(vehicle.getByRole('alert').filter({ hasText: 'Failsafe on link loss:' })).toBeVisible();
  });

dropsOn('Settings');
dropsOn('Plan');

test('reconnecting after a link loss clears the card, the last-seen marker and the old trail, and claims nothing about the lost flight', async ({ vehicle }) => {
  await takeOff(vehicle, '5 m');
  await expect(flightMode(vehicle)).toHaveAccessibleName(/^Hold Flying/, { timeout: 60_000 });
  await showMapFull(vehicle);
  await recordMapSources(vehicle);
  await mapMenuAt(vehicle, 200, -100);
  await vehicle.getByRole('button', { name: 'Go to location' }).click();
  await slideAcross(vehicle, vehicle.getByRole('button', { name: 'Slide to go to location' }));
  await expect.poll(() => mapSource(vehicle, 'aircast-trail'), { timeout: 30_000 }).toContain('LineString');
  await dropLink(vehicle);
  const card = vehicle.getByRole('alert').filter({ hasText: 'Failsafe on link loss:' });
  await expect(card).toBeVisible();
  await expect(vehicle.getByRole('button', { name: /^Link lost How to connect$/ })).toBeVisible();
  await expect.poll(() => mapSource(vehicle, 'aircast-vehicle')).toContain('Last seen');
  expect(await mapSource(vehicle, 'aircast-trail'), 'the trail stays while the aircraft is missing').toContain('LineString');
  await recordNotices(vehicle);
  await connectMock(vehicle);
  await expect(card).toBeHidden();
  await expect(vehicle.getByRole('button', { name: /^Link lost How to connect$/ })).toBeHidden();
  await expect(flightMode(vehicle)).toBeVisible();
  await showMapFull(vehicle);
  await expect.poll(() => mapSource(vehicle, 'aircast-vehicle')).not.toContain('Last seen');
  await expect.poll(() => mapSource(vehicle, 'aircast-trail')).not.toContain('LineString');
  await coreAnswered(vehicle, 3);
  expect((await heard(vehicle)).filter((it) => /disarmed/i.test(it)), 'a different aircraft is not announced as the lost one landing').toEqual([]);
});

const ARM_VOLTAGE = 'vehicle.parameterManager.getParameter(-1,BATT_ARM_VOLT)';

const core = (page: Page, path: string, value?: unknown): Promise<{ valid?: boolean; value?: unknown }> =>
  page.evaluate(
    async ({ module, path, value }) => {
      const { Qgc } = (await import(module)) as { Qgc: { get: (path: string) => Promise<unknown>; set: (path: string, value: unknown) => Promise<unknown> } };
      return (await (value === undefined ? Qgc.get(path) : Qgc.set(path, value))) as { valid?: boolean; value?: unknown };
    },
    { module: '/src/bridge/Qgc.ts', path, value },
  );

test.describe('against ArduPilot', () => {
  const sitl: { running: Sitl | null } = { running: null };

  test.describe.configure({ mode: 'serial' });

  test.beforeAll(async () => {
    test.setTimeout(120_000);
    const running = startSitl();
    sitl.running = running;
    await expect.poll(() => running.heard.heartbeat, { timeout: 90_000 }).toBe(true);
  });

  test.afterAll(() => sitl.running?.stop());

  test('a takeoff the aircraft refuses to arm for says why, once', async ({ app }) => {
    test.setTimeout(300_000);
    await app.addLocatorHandler(app.getByText(/ before this vehicle can fly\.$/), () => app.getByRole('button', { name: 'Later', exact: true }).click());
    await connectSitl(app, sitl.running?.ws ?? '');
    await expect.poll(async () => (await core(app, 'vehicle.homePosition')).valid, { timeout: 180_000 }).toBe(true);
    await expect
      .poll(
        async () => {
          await core(app, ARM_VOLTAGE, 30);
          return (await core(app, ARM_VOLTAGE)).value;
        },
        { timeout: 60_000 },
      )
      .toBe(30);
    await app.getByRole('button', { name: 'Take off', exact: true }).click();
    await app.getByRole('button', { name: '5 m', exact: true }).click();
    await slideAcross(app, app.getByRole('button', { name: /^Slide to take off( anyway)? · / }));
    await expect(app.getByText("Can't arm: battery 1 below minimum arming voltage.")).toBeVisible({ timeout: 30_000 });
    await expect(app.getByText(/^(Can't arm|The aircraft refused to arm|The aircraft didn't answer|The aircraft accepted the arm)/)).toHaveCount(1);
    await expect(flightMode(app)).not.toHaveAccessibleName(/Flying/);
  });
});

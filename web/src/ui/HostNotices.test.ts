import { describe, expect, it, vi } from 'vitest';
import { parseJson } from '@/bridge/Json';
import type { SnackbarDuration, SnackbarResult } from '@/m3';
import { ADDITIONAL_ERRORS, AppMessage, REBOOT_VEHICLE_ACTION, REPEAT_QUIET_MS, criticalBanner, hostNoticesPath, noticeBatch, quietBanners, showLatest } from './HostNotices';

const view = (unseen: string, destination: string | null = null, banners = '') =>
  parseJson(`{"unseen":[${unseen}],"destination":${destination === null ? 'null' : `"${destination}"`},"banners":[${banners}]}`);

describe('HostNoticesTest', () => {
  it('nothing unseen is no batch', () => {
    expect(noticeBatch(null)).toBeNull();
    expect(noticeBatch(view(''))).toBeNull();
    expect(noticeBatch(view('{"kind":"message","title":"no id"}'))).toBeNull();
  });

  it("a batch acknowledges through its latest id and carries the core's rules", () => {
    const batch = noticeBatch(view('{"id":4,"kind":"navigation","known":true},{"id":6,"kind":"somethingNew","known":false}', 'plan', '"Battery · Low voltage"'))!;
    expect(batch.through).toBe(6);
    expect(batch.destination).toBe('plan');
    expect(batch.banners).toEqual(['Battery · Low voltage']);
    expect(batch.unknownKinds).toEqual(['somethingNew']);
    expect(hostNoticesPath(batch.through)).toBe('view.hostNotices(6)');
  });

  it('a banner shown in the last thirty seconds is held back and the rest are kept once', () => {
    const shownAt = new Map([['Battery · Low voltage', 1_000]]);
    expect(quietBanners(['Battery · Low voltage', 'GPS lost', 'GPS lost'], shownAt, 20_000)).toEqual(['GPS lost']);
    expect(quietBanners(['Battery · Low voltage'], shownAt, 1_000 + REPEAT_QUIET_MS)).toEqual(['Battery · Low voltage']);
  });

  it('app messages arrive as OK dialogs, as showAppMessage opens one', () => {
    const batch = noticeBatch(parseJson('{"unseen":[{"id":4,"kind":"message"}],"banners":[],"dialogs":[{"title":"Aircast QGC","text":"Parameters missing","action":null},{"title":"Aircast QGC","text":"Reboot","action":"rebootVehicle"}]}'));
    expect(batch?.dialogs).toEqual([AppMessage({ title: 'Aircast QGC', text: 'Parameters missing' }), AppMessage({ title: 'Aircast QGC', text: 'Reboot', action: REBOOT_VEHICLE_ACTION })]);
  });

  it('the newest payload result comes apart from the queued banners, and a blank one only clears', () => {
    const batch = (payload: string) => noticeBatch(parseJson(`{"unseen":[{"id":3,"kind":"message"}],"banners":[],"payload":${payload}}`));
    expect(batch('"Bay 2 dropped"')?.payload).toBe('Bay 2 dropped');
    expect(batch('""')?.payload).toBe('');
    expect(batch('null')?.payload).toBeNull();
  });

  it('a second error in one batch is still critical and says more arrived', () => {
    expect(criticalBanner([])).toBeNull();
    expect(criticalBanner(['EKF failure'])).toBe('EKF failure');
    expect(criticalBanner(['EKF failure', 'GPS glitch'])).toBe(`EKF failure · ${ADDITIONAL_ERRORS}`);
  });
});

describe('BannersToShowTest', () => {
  const now = 1_000_000;
  const missing = 'Aircast · Parameters are missing';
  const ekf = 'Aircast · EKF variance';

  it('a message arriving many times in one batch is shown once', () => {
    expect(quietBanners([missing, missing], new Map(), now)).toEqual([missing]);
  });

  it('a message that repeats faster than the quiet window is held back', () => {
    expect(quietBanners([missing], new Map([[missing, now - 1_000]]), now)).toEqual([]);
  });

  it('a condition still recurring after the quiet window is announced again', () => {
    expect(quietBanners([ekf], new Map([[ekf, now - REPEAT_QUIET_MS - 1]]), now)).toEqual([ekf]);
  });

  it('one message being held back does not hold back a different one', () => {
    expect(quietBanners([missing, ekf], new Map([[missing, now - 1_000]]), now)).toEqual([ekf]);
  });
});

describe('PayloadNoticeTest', () => {
  const host = () => ({ showSnackbar: vi.fn((_message: string, _actionLabel?: string | null, _duration?: SnackbarDuration, _signal?: AbortSignal) => Promise.resolve<SnackbarResult>('Dismissed')) });

  it('a newer payload result withdraws the one showing or queued and is shown in its place', () => {
    const snackbars = host();
    const first = showLatest(snackbars, null, 'Bay 1 dropped');
    const second = showLatest(snackbars, first, 'Bay 2 dropped');
    expect([first.signal.aborted, second.signal.aborted]).toEqual([true, false]);
    expect(snackbars.showSnackbar.mock.calls.map(([message, , , signal]) => [message, signal === second.signal])).toEqual([
      ['Bay 1 dropped', false],
      ['Bay 2 dropped', true],
    ]);
  });

  it('a blank payload notice clears the one showing and shows nothing', () => {
    const snackbars = host();
    const dropped = showLatest(snackbars, null, 'Payload dropped');
    showLatest(snackbars, dropped, '');
    expect(dropped.signal.aborted).toBe(true);
    expect(snackbars.showSnackbar).toHaveBeenCalledTimes(1);
  });
});

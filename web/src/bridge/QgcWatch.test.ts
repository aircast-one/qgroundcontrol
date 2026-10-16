import { afterEach, describe, expect, it } from 'vitest';
import { Qgc } from './Qgc';

const log = { sent: [] as string[] };
const original = Qgc.sendWatch;

const succeeding = () => {
  Qgc.sendWatch = (csv) => {
    log.sent = [...log.sent, csv];
  };
};

const failing = (reason: Error) => {
  Qgc.sendWatch = () => {
    throw reason;
  };
};

const fresh = () => {
  Qgc.forgetWatchesForTest();
  log.sent = [];
};

describe('QgcWatchTest', () => {
  afterEach(() => {
    Qgc.sendWatch = original;
    Qgc.forgetWatchesForTest();
  });

  it('a path registered before the natives exist is retried, not lost forever', () => {
    fresh();
    failing(new Error('No implementation found for QGCBridge.watch'));
    Qgc.watch(['vehicle.armed']);
    succeeding();
    Qgc.watch(['vehicle.armed']);
    expect(log.sent).toEqual(['vehicle.armed']);
  });

  it('a path that registered once is not sent again', () => {
    fresh();
    succeeding();
    Qgc.watch(['vehicle.armed']);
    Qgc.watch(['vehicle.armed']);
    expect(log.sent.length).toBe(1);
  });

  it('a later path is registered alongside the earlier ones', () => {
    fresh();
    succeeding();
    Qgc.watch(['a']);
    Qgc.watch(['b']);
    expect(log.sent).toEqual(['a', 'a,b']);
  });

  it('a failure does not discard paths that already registered', () => {
    fresh();
    succeeding();
    Qgc.watch(['a']);
    failing(new Error('bridge went away'));
    Qgc.watch(['b']);
    succeeding();
    Qgc.watch(['b']);
    expect(log.sent).toEqual(['a', 'a,b']);
  });

  it('a path is only dropped once its last holder releases it', () => {
    fresh();
    succeeding();
    Qgc.watch(['a']);
    Qgc.watch(['a']);
    Qgc.unwatch(['a']);
    expect(Qgc.watchedPathsForTest()).toEqual(new Set(['a']));
    Qgc.unwatch(['a']);
    expect(Qgc.watchedPathsForTest()).toEqual(new Set());
    expect(log.sent).toEqual(['a', '']);
  });

  it('releasing one path leaves the others registered', () => {
    fresh();
    succeeding();
    Qgc.watch(['a']);
    Qgc.watch(['b']);
    Qgc.unwatch(['a']);
    expect(Qgc.watchedPathsForTest()).toEqual(new Set(['b']));
    expect(log.sent).toEqual(['a', 'a,b', 'b']);
  });

  it('releasing a path that was never registered changes nothing', () => {
    fresh();
    succeeding();
    Qgc.watch(['a']);
    Qgc.unwatch(['never']);
    expect(Qgc.watchedPathsForTest()).toEqual(new Set(['a']));
    expect(log.sent).toEqual(['a']);
  });

  it('a failed release keeps the path registered rather than silently dropping it', () => {
    fresh();
    succeeding();
    Qgc.watch(['a']);
    failing(new Error('bridge went away'));
    Qgc.unwatch(['a']);
    expect(Qgc.watchedPathsForTest()).toEqual(new Set(['a']));
    succeeding();
    Qgc.unwatch(['a']);
    expect(log.sent).toEqual(['a', '']);
  });
});

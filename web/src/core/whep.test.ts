// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { HIDDEN_GRACE_MS, whileShown } from './whep';

const page = { state: 'visible' as DocumentVisibilityState };

const show = (state: DocumentVisibilityState) => {
  page.state = state;
  document.dispatchEvent(new Event('visibilitychange'));
};

const stream = () => {
  const counts = { started: 0, stopped: 0 };
  const start = () => {
    counts.started += 1;
    return () => {
      counts.stopped += 1;
    };
  };
  return { counts, start };
};

describe('whileShown', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => page.state });
    page.state = 'visible';
  });
  afterEach(() => vi.useRealTimers());

  it('rides out a quick tab switch, hangs up a tab hidden past the grace and reconnects the moment it is shown', () => {
    const { counts, start } = stream();
    const stop = whileShown(start, () => false);
    show('hidden');
    vi.advanceTimersByTime(HIDDEN_GRACE_MS - 1);
    show('visible');
    expect(counts).toEqual({ started: 1, stopped: 0 });
    show('hidden');
    vi.advanceTimersByTime(HIDDEN_GRACE_MS);
    expect(counts).toEqual({ started: 1, stopped: 1 });
    show('visible');
    expect(counts).toEqual({ started: 2, stopped: 1 });
    stop();
    expect(counts).toEqual({ started: 2, stopped: 2 });
  });

  it('keeps a recorded stream while hidden and hangs up once the recording ends', () => {
    const { counts, start } = stream();
    const kept = { recording: true };
    whileShown(start, () => kept.recording);
    show('hidden');
    vi.advanceTimersByTime(3 * HIDDEN_GRACE_MS);
    expect(counts.stopped).toBe(0);
    kept.recording = false;
    vi.advanceTimersByTime(HIDDEN_GRACE_MS);
    expect(counts.stopped).toBe(1);
  });

  it('does not open a stream for a tab that loads hidden until it is shown', () => {
    const { counts, start } = stream();
    page.state = 'hidden';
    whileShown(start, () => false);
    expect(counts.started).toBe(0);
    show('visible');
    expect(counts.started).toBe(1);
  });
});

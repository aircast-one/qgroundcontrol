import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { latestWins } from './deviceSettings';

const RETRY_MS = 1_000;

const controlled = () => {
  const sent: string[] = [];
  const answers: ((ok: boolean) => void)[] = [];
  const send = (text: string) => {
    sent.push(text);
    return new Promise<boolean>((resolve) => answers.push(resolve));
  };
  const answer = async (ok: boolean) => {
    answers.shift()?.(ok);
    await vi.advanceTimersByTimeAsync(0);
  };
  return { sent, send, answer };
};

describe('latestWins', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('sends one save at a time and skips straight to the newest text', async () => {
    const device = controlled();
    const save = latestWins(device.send, RETRY_MS);
    void save('a');
    void save('b');
    void save('c');
    expect(device.sent).toEqual(['a']);
    await device.answer(true);
    expect(device.sent).toEqual(['a', 'c']);
    await device.answer(true);
    await vi.advanceTimersByTimeAsync(RETRY_MS);
    expect(device.sent).toEqual(['a', 'c']);
  });

  it('retries a text the device did not take until it does', async () => {
    const device = controlled();
    const save = latestWins(device.send, RETRY_MS);
    void save('a');
    await device.answer(false);
    expect(device.sent).toEqual(['a']);
    await vi.advanceTimersByTimeAsync(RETRY_MS);
    expect(device.sent).toEqual(['a', 'a']);
    await device.answer(true);
    await vi.advanceTimersByTimeAsync(RETRY_MS);
    expect(device.sent).toEqual(['a', 'a']);
  });

  it('a newer text goes out instead of one that failed while it waited', async () => {
    const device = controlled();
    const save = latestWins(device.send, RETRY_MS);
    void save('a');
    void save('b');
    await device.answer(false);
    expect(device.sent).toEqual(['a', 'b']);
    await device.answer(true);
    await vi.advanceTimersByTimeAsync(RETRY_MS);
    expect(device.sent).toEqual(['a', 'b']);
  });
});

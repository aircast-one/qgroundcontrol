// @vitest-environment jsdom
import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useSnackbarHostState } from './Feedback';

const settle = () => act(async () => undefined);

describe('SnackbarHostTest', () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('withdrawing the snackbar on screen hands the host to the next one at once', async () => {
    const { result } = renderHook(() => useSnackbarHostState());
    const withdrawn = new AbortController();
    const first = result.current.showSnackbar('Bay 1 dropped', null, 'Short', withdrawn.signal);
    void result.current.showSnackbar('Bay 2 dropped');
    await settle();
    expect(result.current.current?.message).toBe('Bay 1 dropped');
    withdrawn.abort();
    await settle();
    expect(result.current.current?.message).toBe('Bay 2 dropped');
    await expect(first).resolves.toBe('Dismissed');
  });

  it('a withdrawn snackbar still waiting is never shown, and the others keep their turn', async () => {
    const { result } = renderHook(() => useSnackbarHostState());
    void result.current.showSnackbar('Sticks took over: Loiter');
    const withdrawn = new AbortController();
    const skipped = result.current.showSnackbar('Bay 1 dropped', null, 'Short', withdrawn.signal);
    void result.current.showSnackbar('Bay 2 dropped');
    await settle();
    withdrawn.abort();
    await settle();
    expect(result.current.current?.message).toBe('Sticks took over: Loiter');
    await act(async () => vi.advanceTimersByTime(4_000));
    expect(result.current.current?.message).toBe('Bay 2 dropped');
    await expect(skipped).resolves.toBe('Dismissed');
  });
});

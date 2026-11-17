// @vitest-environment jsdom
import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { SnackbarHostState } from '@/m3';
import { type ChangeNotice, useChangeNotice } from '@/ui/ValueRow';

const host = (result: 'ActionPerformed' | 'Dismissed'): SnackbarHostState & { calls: unknown[][] } => {
  const calls: unknown[][] = [];
  return { calls, current: null, showSnackbar: (...args: unknown[]) => Promise.resolve(void calls.push(args)).then(() => result) };
};

const noticeFor = (snackbars: SnackbarHostState): ChangeNotice => renderHook(() => useChangeNotice(snackbars)).result.current;

describe('a parameter write anywhere in the app is announced with Undo', () => {
  afterEach(cleanup);

  it('offers Undo and runs it when the operator presses it', async () => {
    const snackbars = host('ActionPerformed');
    const notice = noticeFor(snackbars);
    const undo = vi.fn(() => Promise.resolve(null));
    await act(async () => notice.show('RTL altitude set to 20 m', undo));
    expect(snackbars.calls[0]).toEqual(['RTL altitude set to 20 m', 'Undo', 'Long']);
    expect(undo).toHaveBeenCalledOnce();
  });

  it('says when the undo itself was refused', async () => {
    const snackbars = host('ActionPerformed');
    const notice = noticeFor(snackbars);
    await act(async () => notice.show('RTL altitude set to 20 m', () => Promise.resolve('the vehicle is armed')));
    expect(snackbars.calls[1]).toEqual(['Undo failed: the vehicle is armed']);
  });

  it('leaves the value alone when the toast is dismissed', async () => {
    const snackbars = host('Dismissed');
    const notice = noticeFor(snackbars);
    const undo = vi.fn(() => Promise.resolve(null));
    await act(async () => notice.show('RTL altitude set to 20 m', undo));
    expect(undo).not.toHaveBeenCalled();
  });
});

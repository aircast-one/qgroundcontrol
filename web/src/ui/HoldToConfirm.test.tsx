// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { DECK_HOLD_MS } from '@/ui/FlyDeck';
import { HoldToConfirm } from './HoldToConfirm';

const advanceTimeBy = (milliseconds: number) => act(() => vi.advanceTimersByTime(milliseconds));

const down = (label: string) => fireEvent.pointerDown(screen.getByText(label), { button: 0, pointerId: 1, clientX: 10 });

const up = (label: string) => fireEvent.pointerUp(screen.getByText(label), { button: 0, pointerId: 1, clientX: 10 });

describe('HoldToConfirmUiTest', () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'requestAnimationFrame', 'cancelAnimationFrame', 'performance'] });
    Object.defineProperty(HTMLElement.prototype, 'setPointerCapture', { value: () => {}, configurable: true });
  });

  afterEach(() => {
    cleanup();
    Reflect.deleteProperty(HTMLElement.prototype, 'setPointerCapture');
    vi.useRealTimers();
  });

  it('holding for the full time confirms once', () => {
    const fired = vi.fn();
    render(<HoldToConfirm label="Slide to land" onConfirm={fired} />);
    down('Slide to land');
    advanceTimeBy(DECK_HOLD_MS + 200);
    up('Slide to land');
    advanceTimeBy(100);
    expect(fired).toHaveBeenCalledTimes(1);
  });

  it('a hold whose action changes mid-press never fires the new action', () => {
    const fired = vi.fn();
    const view = render(<HoldToConfirm label="Slide to take off" onConfirm={fired} />);
    down('Slide to take off');
    advanceTimeBy(DECK_HOLD_MS / 2);
    view.rerender(<HoldToConfirm label="Slide to take off anyway" onConfirm={fired} />);
    advanceTimeBy(DECK_HOLD_MS * 2);
    up('Slide to take off anyway');
    advanceTimeBy(100);
    expect(fired, 'the pilot let go after reading the warning, so nothing is sent').not.toHaveBeenCalled();
  });
});

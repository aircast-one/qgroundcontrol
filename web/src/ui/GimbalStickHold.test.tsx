// @vitest-environment jsdom
import { act, cleanup, fireEvent, render } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { GIMBAL_DRAG_REPEAT_MS, GimbalScreenControl } from '@/ui/GimbalIndicator';

const shown = vi.hoisted(() => ({ view: '' }));
vi.mock('@/bridge/QgcState', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/bridge/QgcState')>()), useQgcPath: () => (shown.view === '' ? null : parseJson(shown.view)) }));

const HOLD = '{"shown":true,"onScreen":{"enabled":true,"clickAndDrag":true,"stick":false}}';
const BOX = { left: 0, top: 0, width: 200, height: 100, right: 200, bottom: 100, x: 0, y: 0, toJSON: () => ({}) };

describe('a hold on the picture never outlives the finger that made it', () => {
  const sticks = () => vi.mocked(Qgc.call).mock.calls.filter(([path]) => path === 'gimbal.stick').map(([, x, y]) => [x, y]);
  const pad = (container: HTMLElement) => {
    const element = container.firstElementChild as HTMLDivElement;
    element.setPointerCapture = () => undefined;
    element.getBoundingClientRect = () => BOX;
    return element;
  };
  const held = () => {
    const rendered = render(<GimbalScreenControl />);
    const element = pad(rendered.container);
    fireEvent.pointerDown(element, { pointerId: 1, clientX: 100, clientY: 50 });
    fireEvent.pointerMove(element, { pointerId: 1, clientX: 150, clientY: 50 });
    act(() => void vi.advanceTimersByTime(GIMBAL_DRAG_REPEAT_MS * 2));
    return { ...rendered, element };
  };
  const quietAfter = (stop: () => void) => {
    stop();
    const stopped = sticks();
    act(() => void vi.advanceTimersByTime(GIMBAL_DRAG_REPEAT_MS * 5));
    return { stopped, later: sticks() };
  };

  beforeEach(() => {
    vi.useFakeTimers();
    shown.view = HOLD;
    vi.spyOn(Qgc, 'call').mockResolvedValue(null);
    vi.spyOn(Qgc, 'watch').mockImplementation(() => undefined);
  });
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('a held deflection is resent while the finger is down, so the core knows the head is still there', () => {
    held();
    expect(sticks().length).toBeGreaterThanOrEqual(2);
    expect(sticks().at(-1)).toEqual([0.5, 0]);
  });

  it('a press that never moves sends nothing, so a tap does not take the gimbal or the camera channels', () => {
    const { container } = render(<GimbalScreenControl />);
    const element = pad(container);
    fireEvent.pointerDown(element, { pointerId: 1, clientX: 100, clientY: 50 });
    act(() => void vi.advanceTimersByTime(GIMBAL_DRAG_REPEAT_MS * 3));
    fireEvent.pointerUp(element, { pointerId: 1 });
    expect(sticks()).toEqual([]);
  });

  it.each([
    ['the pointer is cancelled', (element: HTMLElement) => fireEvent.pointerCancel(element, { pointerId: 1 })],
    ['the window loses focus', () => fireEvent.blur(window)],
    ['the page is put away', () => fireEvent(window, new Event('pagehide'))],
    [
      'the tab is hidden',
      () => {
        vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('hidden');
        fireEvent(document, new Event('visibilitychange'));
      },
    ],
  ])('the turn stops once when %s', (_name, stop) => {
    const { element } = held();
    const { stopped, later } = quietAfter(() => stop(element));
    expect(stopped.at(-1)).toEqual([0, 0]);
    expect(later).toEqual(stopped);
  });

  it.each([
    ['the gimbal view drops out', ''],
    ['the mode flips to the joystick', '{"shown":true,"onScreen":{"enabled":true,"clickAndDrag":true,"stick":true}}'],
    ['the mode flips to tap to point', '{"shown":true,"onScreen":{"enabled":true,"clickAndDrag":false,"stick":false}}'],
  ])('the turn stops once when %s mid-hold', (_name, view) => {
    const { rerender } = held();
    const { stopped, later } = quietAfter(() => {
      shown.view = view;
      rerender(<GimbalScreenControl />);
    });
    expect(stopped.at(-1)).toEqual([0, 0]);
    expect(later).toEqual(stopped);
  });

  it('a second finger neither restarts the hold nor ends it when it lifts', () => {
    const { element } = held();
    fireEvent.pointerDown(element, { pointerId: 2, clientX: 10, clientY: 10 });
    fireEvent.pointerUp(element, { pointerId: 2 });
    act(() => void vi.advanceTimersByTime(GIMBAL_DRAG_REPEAT_MS * 2));
    expect(sticks().at(-1)).toEqual([0.5, 0]);
    fireEvent.pointerUp(element, { pointerId: 1 });
    expect(sticks().at(-1)).toEqual([0, 0]);
  });
});

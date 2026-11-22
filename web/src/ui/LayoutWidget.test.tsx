// @vitest-environment jsdom
import { act, render } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { getSharedPreferences } from '@/kotlin/Platform';
import { useValue } from '@/kotlin/State';
import { LocalAvoidedByVideoMessage } from '@/ui/FlyShell';
import { FlyScreenState, LocalFlyScreenState } from '@/ui/FlyScreenState';
import { LayoutGroup, LayoutWidget, OverlayLayoutState } from '@/ui/OverlayLayout';

const RESTING = { left: 1093, top: 659, right: 1225, bottom: 709 };

const box = ({ left, top, right, bottom }: typeof RESTING): DOMRect => ({ left, top, right, bottom, x: left, y: top, width: right - left, height: bottom - top, toJSON: () => ({}) });

const animated = (element: HTMLElement): boolean => element.style.getPropertyValue('--aircast-jiggle') !== '' || element.style.getPropertyValue('scale') !== '';

describe('a layout item jiggling in Edit layout over the picture', () => {
  const sampled = { count: 0 };

  beforeEach(() => {
    sampled.count = 0;
    vi.stubGlobal('matchMedia', () => ({ matches: false, addEventListener: () => undefined, removeEventListener: () => undefined }));
    vi.stubGlobal(
      'ResizeObserver',
      class {
        constructor(private readonly seen: () => void) {}
        observe() {
          this.seen();
        }
        disconnect() {}
      },
    );
    vi.spyOn(HTMLElement.prototype, 'offsetWidth', 'get').mockReturnValue(132);
    vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockReturnValue(50);
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (this: HTMLElement) {
      if (!animated(this)) return box(RESTING);
      sampled.count += 1;
      return box({ ...RESTING, right: RESTING.right + sampled.count / 100, bottom: RESTING.bottom + sampled.count / 100 });
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  const Shell = ({ flyScreen, grouped }: { flyScreen: FlyScreenState; grouped: boolean }) => {
    useValue(flyScreen.obstacles);
    const item = (
      <LayoutWidget widgetKey="cameraSwitch">
        <button type="button">Cameras</button>
      </LayoutWidget>
    );
    return (
      <LocalFlyScreenState.Provider value={flyScreen}>
        <LocalAvoidedByVideoMessage.Provider value>{grouped ? <LayoutGroup widgetKey="readings">{item}</LayoutGroup> : item}</LocalAvoidedByVideoMessage.Provider>
      </LocalFlyScreenState.Provider>
    );
  };

  it.each([false, true])('stands as an obstacle where it rests, so the picture keeps drawing as it jiggles (inside a group: %s)', (grouped) => {
    const flyScreen = new FlyScreenState(new OverlayLayoutState(getSharedPreferences('layout-widget-test')));
    render(<Shell flyScreen={flyScreen} grouped={grouped} />);
    act(() => {
      flyScreen.layout.editing.value = true;
    });
    act(() => {
      flyScreen.layout.hidden.value = new Set(['other']);
    });
    expect(flyScreen.obstacles.get('cameraSwitch')).toMatchObject(RESTING);
  });
});

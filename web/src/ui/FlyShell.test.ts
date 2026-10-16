import { describe, expect, it } from 'vitest';
import { FlyView, FlyViewEntries, Letterbox, MiniMap, flyViewAllowed, flyViewsOffered, miniMapNamed, stickInsets } from './FlyShell';

describe('PipRulesTest', () => {
  it("the 3D view is offered only while QGC's 3D viewer is enabled, or already showing", () => {
    expect(flyViewsOffered(false, FlyView.Video)).toEqual([FlyView.Video, FlyView.Map]);
    expect(flyViewsOffered(true, FlyView.Video)).toEqual(FlyViewEntries);
    expect(flyViewsOffered(false, FlyView.ThreeD)).toEqual([FlyView.Video, FlyView.Map]);
    expect(flyViewsOffered(null, FlyView.ThreeD)).toEqual(FlyViewEntries);
    expect(flyViewAllowed(false, FlyView.ThreeD)).toBe(FlyView.Map);
    expect(flyViewAllowed(null, FlyView.ThreeD)).toBe(FlyView.ThreeD);
    expect(flyViewAllowed(false, FlyView.Video)).toBe(FlyView.Video);
  });
});

describe('StickInsetsTest', () => {
  const pad = 172;

  const gaps = (letterbox: Letterbox, controls: number, fullScreen = false): readonly [number, number] => {
    const insets = stickInsets(pad, fullScreen, letterbox, 0, 0, controls);
    const trailing = fullScreen ? 0 : controls + insets.trailingGap;
    return [letterbox.start + insets.rowStart, letterbox.end + insets.rowEnd + trailing];
  };

  it('both sticks sit the same distance from their glass edge when the camera column is the wider side', () => {
    expect(gaps(Letterbox({ start: 52, end: 0 }), 144)).toEqual([164, 164]);
  });

  it('the action rail sets the inset when the camera column is narrow, and the right stick matches it', () => {
    expect(gaps(Letterbox({ start: 52, end: 0 }), 40)).toEqual([124, 124]);
  });

  it('full screen mirrors the sticks across a letterboxed cutout band', () => {
    expect(gaps(Letterbox({ start: 52, end: 0 }), 144, true)).toEqual([64, 64]);
  });

  it('without sticks the overlays keep the rail clearance and the map keeps its ornaments', () => {
    const insets = stickInsets(null, false, Letterbox({ start: 0, end: 0 }), 0, 0, 0);
    expect(insets.overlaysStart).toBe(72);
    expect(insets.overlaysEnd).toBe(12);
    expect(insets.ornamentEnd).toBeNull();
  });
});

describe('DjiChromeTest', () => {
  it('the mini-map opens as a thumbnail unless the pilot chose otherwise', () => {
    expect(miniMapNamed(null)).toBe(MiniMap.Thumb);
    expect(miniMapNamed('Compass')).toBe(MiniMap.Compass);
    expect(miniMapNamed('Unknown')).toBe(MiniMap.Thumb);
  });
});

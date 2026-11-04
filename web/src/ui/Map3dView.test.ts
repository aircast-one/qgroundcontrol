import { describe, expect, it } from 'vitest';
import { LOOKING_TITLE } from '@/ui/ConnectingCard';
import { Offset, Rect } from '@/ui/FlyShell';
import { map3dControlsAt, map3dHint, map3dNote, map3dOpenAt, map3dPickArgs, map3dState, map3dToggleLabel, map3dToggleText, mouseTurn, pinchStep, wheelScale } from '@/ui/Map3dView';
import { SyntheticDraw } from '@/ui/SyntheticView';

describe('Map3dNoteTest', () => {
  const following = map3dState({ following: true });
  const resting = map3dState({ following: false });

  it('the 3D map looks for the aircraft only while it follows one that is not placed', () => {
    expect(map3dNote(null, following)).toBe(LOOKING_TITLE);
    expect(map3dNote({ available: false, draw: 'drawn' }, following)).toBe(LOOKING_TITLE);
    expect(map3dNote({ available: false, draw: 'drawn' }, resting)).toBeNull();
    expect(map3dNote({ available: false, draw: 'loading' }, resting)).toBe(SyntheticDraw.Loading.note);
  });

  it('with the aircraft placed the 3D map tells how the drawing goes', () => {
    expect(map3dNote({ available: true, draw: 'loading' }, following)).toBe(SyntheticDraw.Loading.note);
    expect(map3dNote({ available: true, draw: 'failed' }, following)).toBe(SyntheticDraw.Failed.note);
    expect(map3dNote({ available: true, draw: 'drawn' }, resting)).toBeNull();
  });
});

describe('Map3dStateTest', () => {
  it('the 3D map reads its bearing, whether it follows and whether an aircraft is placed', () => {
    expect(map3dState({ heading: 90.5, tilt: 35, range: 500, following: false, aircraft: true })).toEqual({ heading: 90.5, following: false, aircraft: true });
  });

  it('before the core says anything the 3D map faces north and follows, with no aircraft', () => {
    expect(map3dState(null)).toEqual({ heading: 0.0, following: true, aircraft: false });
  });
});

describe('Map3dOpenTest', () => {
  it('the 3D map opens where the 2D map last was, else where it was saved, else with nothing', () => {
    const saved = { centre: { latitude: 47.0, longitude: 8.0 }, zoom: 15.0 };
    expect(map3dOpenAt({ latitude: 47.4, longitude: 8.5 }, saved)).toEqual([47.4, 8.5]);
    expect(map3dOpenAt(null, saved)).toEqual([47.0, 8.0]);
    expect(map3dOpenAt(null, null)).toEqual([]);
  });
});

describe('Map3dControlsTest', () => {
  it('the 3D map controls sit where the fly map keeps its scale bar and logo', () => {
    expect(map3dControlsAt(120, 64)).toEqual({ paddingRight: 64, paddingBottom: 132 });
    expect(map3dControlsAt(120, null)).toEqual({ padding: 12 });
  });
});

describe('Map3dToggleTest', () => {
  it('the toggle offers the map that is not on screen', () => {
    expect([map3dToggleText(false), map3dToggleLabel(false)]).toEqual(['3D', 'Show the 3D map']);
    expect([map3dToggleText(true), map3dToggleLabel(true)]).toEqual(['2D', 'Show the flat map']);
  });
});

describe('Map3dHintTest', () => {
  it('the hint names the gestures of the pointer in hand', () => {
    expect(map3dHint(true)).toBe('Drag to move · Two fingers to turn and tilt');
    expect(map3dHint(false)).toBe('Drag to move · Right-drag or Shift-drag to turn and tilt');
  });
});

describe('Map3dGesturesTest', () => {
  it('two fingers spreading apart zoom in by their spread', () => {
    const step = pinchStep([Offset(100, 100), Offset(200, 100)], [Offset(50, 100), Offset(250, 100)]);
    expect(step.scale).toBeCloseTo(2.0);
    expect(step.turn).toBeCloseTo(0.0);
    expect(step.up).toBeCloseTo(0.0);
  });

  it('two fingers twisting clockwise on screen turn clockwise, across the ±180° seam too', () => {
    expect(pinchStep([Offset(0, 0), Offset(100, 0)], [Offset(0, 0), Offset(0, 100)]).turn).toBeCloseTo(90.0);
    expect(pinchStep([Offset(100, 1), Offset(0, 0)], [Offset(100, -1), Offset(0, 0)]).turn).toBeCloseTo(-1.1458, 3);
  });

  it('two fingers moving up together tilt by how far their midpoint rose', () => {
    expect(pinchStep([Offset(0, 300), Offset(100, 300)], [Offset(0, 260), Offset(100, 240)]).up).toBeCloseTo(50.0);
  });

  it('a mouse dragged across the whole view turns half a circle', () => {
    expect(mouseTurn(800, 800)).toBeCloseTo(180.0);
    expect(mouseTurn(-400, 800)).toBeCloseTo(-90.0);
    expect(mouseTurn(10, 0)).toBeCloseTo(1800.0);
  });

  it('the wheel rolled away zooms in, toward you zooms out, and line deltas count as pixels', () => {
    expect(wheelScale(-100, 0)).toBeGreaterThan(1.0);
    expect(wheelScale(100, 0)).toBeLessThan(1.0);
    expect(wheelScale(100, 0) * wheelScale(-100, 0)).toBeCloseTo(1.0);
    expect(wheelScale(1, 1)).toBeCloseTo(wheelScale(16, 0));
  });
});

describe('Map3dPickTest', () => {
  it('a press asks for the ground under it in the 3D view own pixels, with the view size', () => {
    expect(map3dPickArgs(Offset(410, 320), Rect(10, 20, 810, 620))).toEqual([400, 300, 800, 600]);
    expect(map3dPickArgs(Offset(0, 0), Rect(0, 0, 390, 844))).toEqual([0, 0, 390, 844]);
  });
});

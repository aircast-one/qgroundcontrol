import { describe, expect, it } from 'vitest';
import { LOOKING_TITLE } from '@/ui/ConnectingCard';
import { Offset } from '@/ui/FlyShell';
import { map3dNote, mouseTurn, pinchStep, wheelScale } from '@/ui/Map3dView';
import { SyntheticDraw } from '@/ui/SyntheticView';

describe('Map3dNoteTest', () => {
  it('the 3D map looks for the aircraft until it is placed, then tells how the drawing goes', () => {
    expect(map3dNote(null)).toBe(LOOKING_TITLE);
    expect(map3dNote({ available: false, draw: 'drawn' })).toBe(LOOKING_TITLE);
    expect(map3dNote({ available: true, draw: 'loading' })).toBe(SyntheticDraw.Loading.note);
    expect(map3dNote({ available: true, draw: 'failed' })).toBe(SyntheticDraw.Failed.note);
    expect(map3dNote({ available: true, draw: 'drawn' })).toBeNull();
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

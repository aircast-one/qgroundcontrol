import { describe, expect, it } from 'vitest';
import { covers, holeBackground, overlap, paintedBelow, paintedCells, placementOf, samePlacement, stackOrder, surfacePath } from '@/ui/SyntheticSurface';

const area = (left: number, top: number, width: number, height: number) => ({ left, top, width, height });

describe('SyntheticSurfaceTest', () => {
  it('only the Mac desktop app draws the 3D view natively, Windows keeps the frames and the browser the worker', () => {
    expect(surfacePath(true, true)).toBe('layer');
    expect(surfacePath(true, false)).toBe('jpeg');
    expect(surfacePath(false, true), 'Safari on a Mac runs the core in the page').toBe('worker');
    expect(surfacePath(false, false)).toBe('worker');
  });

  it('the native view sits on the canvas and draws at device pixels', () => {
    expect(placementOf(area(12, 40, 800.5, 600), { width: 800.5, height: 600 }, 2, 868, true)).toEqual({
      x: 12,
      y: 40,
      width: 800.5,
      height: 600,
      scale: 2,
      pixelWidth: 1601,
      pixelHeight: 1200,
      viewportHeight: 868,
      shown: true,
    });
    expect(placementOf(area(0, 0, 400, 300), { width: 200, height: 150 }, 2, 868, true).pixelWidth, 'a canvas scaled up mid-transition keeps drawing at its laid-out size').toBe(400);
    expect(placementOf(area(0, 0, 0, 0), { width: 0, height: 0 }, 2, 868, true).shown, 'a collapsed canvas hides the view').toBe(false);
    expect(placementOf(area(0, 0, 800, 600), { width: 800, height: 600 }, 2, 868, false).shown, 'a canvas under display: none hides the view').toBe(false);
  });

  it('the page only tells the shell when the canvas really moved', () => {
    const placed = placementOf(area(0, 0, 800, 600), { width: 800, height: 600 }, 2, 868, true);
    expect(samePlacement(placed, null)).toBe(false);
    expect(samePlacement(placed, { ...placed })).toBe(true);
    expect(samePlacement(placed, { ...placed, y: 1 })).toBe(false);
    expect(samePlacement(placed, { ...placed, scale: 1 }), 'a move to a non-Retina screen redraws at its pixels').toBe(false);
    expect(samePlacement(placed, { ...placed, viewportHeight: 900 }), 'full screen drops the title bar above the page').toBe(false);
  });

  it('a background around the 3D views keeps its colour everywhere but over them', () => {
    const box = area(0, 0, 1000, 500);
    const sum = (cells: readonly { width: number; height: number }[]) => cells.reduce((total, cell) => total + cell.width * cell.height, 0);
    const clear = (cells: readonly { left: number; top: number; width: number; height: number }[], hole: { left: number; top: number; width: number; height: number }) =>
      cells.every((cell) => overlap(cell, hole).width * overlap(cell, hole).height === 0);
    const one = paintedCells([area(100, 50, 200, 100)], box);
    expect(sum(one)).toBe(1000 * 500 - 200 * 100);
    expect(clear(one, area(100, 50, 200, 100))).toBe(true);
    const split = [area(0, 0, 1000, 200), area(0, 260, 1000, 240)];
    const two = paintedCells(split, box);
    expect(sum(two), 'a portrait split punches the video pane and the 3D map at once').toBe(1000 * 60);
    expect(split.every((hole) => clear(two, hole))).toBe(true);
    expect(paintedCells([area(-50, -50, 2000, 2000)], area(0, 0, 100, 100)), 'a view larger than the box clears all of it').toEqual([]);
    expect(holeBackground('red', [area(10, 10, 100, 100)], area(10, 10, 100, 100)), 'the 3D view’s own frame paints nothing').toBe('none');
    expect(holeBackground('red', [area(0, 0, 100, 50)], area(0, 0, 100, 100))).toBe('linear-gradient(red, red) 0px 50px / 100px 50px no-repeat border-box');
  });

  it('only what the 3D view covers whole is hidden beneath it', () => {
    const hole = area(0, 0, 800, 600);
    expect(covers(hole, area(100, 0, 600, 600)), 'the black video element under the synthetic view').toBe(true);
    expect(covers(hole, area(0.25, 0, 800.25, 600)), 'sub-pixel layout rounding still counts as covered').toBe(true);
    expect(covers(hole, area(700, 0, 200, 600)), 'a box sticking out stays visible').toBe(false);
    expect(covers(hole, area(0, 0, 0, 0)), 'an empty box paints nothing to hide').toBe(false);
  });

  it('only what paints under the 3D view is cleared, whatever its place in the page', () => {
    const map = stackOrder('relative', 'auto');
    expect(paintedBelow(stackOrder('absolute', '2'), map, true), 'the fly chrome comes first in the page but stacks above the map').toBe(false);
    expect(paintedBelow(stackOrder('absolute', 'auto'), map, true), 'the black video element before the synthetic view').toBe(true);
    expect(paintedBelow(stackOrder('absolute', 'auto'), map, false), 'an overlay after it paints on top').toBe(false);
    expect(paintedBelow(stackOrder('static', 'auto'), map, false), 'plain blocks paint before positioned ones').toBe(true);
    expect(paintedBelow(stackOrder('absolute', '-1'), stackOrder('static', 'auto'), false)).toBe(true);
  });

  it('a box is judged by the part its clipping parents let show', () => {
    expect(overlap(area(-100, 0, 1000, 600), area(0, 0, 800, 600)), 'a cropped video is cut to its frame').toEqual(area(0, 0, 800, 600));
    expect(overlap(area(0, 0, 100, 100), area(200, 200, 100, 100))).toEqual(area(200, 200, 0, 0));
  });
});

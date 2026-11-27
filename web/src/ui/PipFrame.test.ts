import { describe, expect, it } from 'vitest';
import { Offset, Size } from './FlyShell';
import { type PipGesture, type PipRoom, type PipShown, gripScale, pipDrawn, pipFollowed, pipJoined, pipKept, pipLargestScale, pipScaleWithin, pipWithin, scaledPip, storedPipPlace, storedPipScale } from './PipFrame';

describe('PipPlaceTest', () => {
  it('a dragged picture in picture stays on screen, right of and above where it starts', () => {
    const room = Size(800, 400);
    expect(pipWithin(Offset(300, -200), room)).toEqual(Offset(300, -200));
    expect(pipWithin(Offset(-50, 30), room), 'not past the start edge or below the bottom').toEqual(Offset(0, 0));
    expect(pipWithin(Offset(5000, -5000), room), 'not off the far edge or into the status row').toEqual(Offset(800, -400));
    expect(pipWithin(Offset(100, -100), Size(-10, -10)), 'a window smaller than the picture keeps it home').toEqual(Offset(0, 0));
  });

  it('the place is remembered as two numbers and anything else starts it home', () => {
    expect(storedPipPlace('300,-200')).toEqual(Offset(300, -200));
    expect(storedPipPlace(null)).toEqual(Offset(0, 0));
    expect(storedPipPlace('300')).toEqual(Offset(0, 0));
    expect(storedPipPlace('a,b')).toEqual(Offset(0, 0));
  });

  it('the picture grows from its grip by however far it is pulled right or up, keeping its shape', () => {
    const base = Size(320, 180);
    expect(gripScale(1, base, 160, 0), 'half its width to the right is half again as big').toBe(1.5);
    expect(gripScale(1, base, 10, -90), 'up counts in heights when it is the bigger pull').toBe(1.5);
    expect(gripScale(1.5, base, -160, 20), 'pulled back left it shrinks').toBe(1);
    expect(scaledPip(base, 1.5)).toEqual(Size(480, 270));
  });

  it('the picture is kept between a small thumbnail and most of the screen, and a stored size outside that is brought back in', () => {
    const base = Size(320, 180);
    const largest = pipLargestScale(base, Size(1200, 600));
    expect(largest, 'seven tenths of the width, unless the height runs out first').toBeCloseTo(2.625);
    expect(pipScaleWithin(10, largest)).toBeCloseTo(2.625);
    expect(pipScaleWithin(0.1, largest), 'never so small it cannot be found or grabbed').toBe(0.6);
    expect(pipScaleWithin(1.2, pipLargestScale(base, Size(100, 50))), 'a window too small to grow it in still leaves the smallest size').toBe(0.6);
    expect(storedPipScale('1.5')).toBe(1.5);
    expect(storedPipScale(null)).toBe(1);
    expect(storedPipScale('nope')).toBe(1);
    expect(storedPipScale('0')).toBe(1);
  });

  const camera: PipRoom = { base: Size(320, 180), space: Size(1200, 600), scalable: true };
  const button: PipRoom = { base: Size(48, 48), space: Size(1200, 600), scalable: false };

  it('dragging the map or synthetic button, which has one size, leaves the saved picture size alone', () => {
    const stored: PipShown = { moved: Offset(100, -100), scale: 1.5 };
    const drawn = pipDrawn(stored, button);
    expect(drawn.scale, 'the button itself is never scaled').toBe(1);
    const kept = pipKept({ moved: Offset(400, -300), scale: drawn.scale }, drawn, stored, button);
    expect(kept).toEqual({ moved: Offset(400, -300), scale: 1.5 });
    expect(pipDrawn(kept, camera).scale, 'the camera picture comes back at the size it was left').toBe(1.5);
  });

  it('moving the picture in a window too small for its saved size does not shrink it for good', () => {
    const stored: PipShown = { moved: Offset(0, 0), scale: 2 };
    const narrow: PipRoom = { ...camera, space: Size(500, 300) };
    const drawn = pipDrawn(stored, narrow);
    expect(drawn.scale).toBeLessThan(2);
    expect(pipKept({ moved: Offset(20, -10), scale: drawn.scale }, drawn, stored, narrow).scale, 'only shown smaller, still saved at 2').toBe(2);
  });

  it('a pinch or the grip saves the new size, kept within what fits, and leaves the saved place alone', () => {
    const stored: PipShown = { moved: Offset(5000, 0), scale: 1 };
    const drawn = pipDrawn(stored, camera);
    expect(pipKept({ moved: drawn.moved, scale: 1.4 }, drawn, stored, camera)).toEqual({ moved: Offset(5000, 0), scale: 1.4 });
    expect(pipKept({ moved: drawn.moved, scale: 40 }, drawn, stored, camera).scale).toBeCloseTo(2.625);
  });

  it('a drag starts from where the picture is drawn, not from a saved place the room no longer reaches', () => {
    const stored: PipShown = { moved: Offset(1100, -500), scale: 1 };
    const drawn = pipDrawn(stored, camera);
    expect(drawn.moved, 'saved while it was the 48 px button, the 320 px picture is drawn short of it').toEqual(Offset(880, -420));
    const move: PipGesture = { kind: 'move', from: Offset(900, 100), start: drawn };
    const pulled = pipFollowed(move, new Map([[1, Offset(880, 110)]]), Offset(880, 110), false, camera.base);
    expect(pulled?.moved, 'twenty pixels of pointer move it twenty pixels at once').toEqual(Offset(860, -410));
    expect(pipFollowed(move, new Map([[1, Offset(903, 102)]]), Offset(903, 102), false, camera.base), 'a press that barely moves is still a tap').toBeNull();
    expect(pipFollowed(move, new Map([[1, Offset(903, 102)]]), Offset(903, 102), true, camera.base)?.moved, 'once dragging it follows every pixel').toEqual(Offset(883, -418));
  });

  it('a second finger anywhere on screen turns a held picture into a pinch, but not the one-size button or the grip', () => {
    const start: PipShown = { moved: Offset(0, 0), scale: 1 };
    const held: PipGesture = { kind: 'move', from: Offset(40, 560), start };
    const fingers = new Map([[1, Offset(40, 560)], [2, Offset(340, 160)]]);
    const pinch = pipJoined(held, fingers, null, true);
    expect(pinch).toEqual({ kind: 'pinch', spread: 500, start });
    const wider = new Map([[1, Offset(40, 560)], [2, Offset(640, -240)]]);
    expect(pipFollowed(pinch, wider, Offset(640, -240), false, camera.base), 'twice the spread, twice the size').toEqual({ moved: Offset(0, 0), scale: 2 });
    expect(pipJoined(held, fingers, null, false), 'the button has one size').toBe(held);
    const grip: PipGesture = { kind: 'grip', from: Offset(300, 400), start };
    expect(pipJoined(grip, fingers, null, true)).toBe(grip);
    expect(pipJoined(held, fingers, { moved: Offset(30, -30), scale: 1 }, true), 'a pinch that follows a drag keeps where the drag got to').toEqual({ kind: 'pinch', spread: 500, start: { moved: Offset(30, -30), scale: 1 } });
  });
});

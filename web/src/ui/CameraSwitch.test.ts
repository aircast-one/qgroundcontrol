import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { camerasReading } from './Cameras';
import { cameraSwitchState, chipCamera, neighbourCamera, pickedReading, pipCamera, splitToggleTarget } from './CameraSwitch';
import { SYNTHETIC_SOURCE, syntheticChosen } from './SyntheticView';

const json = (text: string): Json => parseJson(text) ?? {};

describe('CameraSwitchTest', () => {
  const camera = (slot: number, active = false, problem: string | null = null) =>
    `{"slot":${slot},"title":"Camera ${slot + 1}","short":"Cam ${slot + 1}","active":${active},"status":"live","problem":${problem === null ? 'null' : `"${problem}"`}}`;

  const reading = (cameras: readonly string[], pip = '{"enabled":false,"slot":null}') => camerasReading(json(`{"class":"Cameras","pip":${pip},"cameras":[${cameras.join(',')}]}`));

  it('the picker hides until there are two cameras to choose between', () => {
    expect(cameraSwitchState(null)).toBeNull();
    expect(cameraSwitchState(reading([camera(0, true)]))).toBeNull();
    expect(cameraSwitchState(reading([camera(0, true), camera(1, false, 'No address')]))).toBeNull();
  });

  it('the picker lists every camera that can play, the one on screen chosen', () => {
    const state = cameraSwitchState(reading([camera(0, true, 'No address'), camera(1), camera(2, false, 'No address'), camera(3)]));
    expect(state?.cameras.map((it) => it.slot)).toEqual([0, 1, 3]);
    expect(state?.shown.slot).toBe(0);
    expect(cameraSwitchState(reading([camera(0), camera(1)]))?.shown.slot).toBe(0);
  });

  it('the second camera shows only when the split is switched on and there is one', () => {
    const cameras = [camera(0, true), camera(1)];
    expect(pipCamera(reading(cameras, '{"enabled":true,"slot":1}'), true)?.slot).toBe(1);
    expect(pipCamera(reading(cameras, '{"enabled":false,"slot":1}'), true)).toBeNull();
    expect(pipCamera(reading(cameras, '{"enabled":true,"slot":null}'), true)).toBeNull();
  });

  it('the split button flips the setting, and hides with no second camera', () => {
    const cameras = [camera(0, true), camera(1)];
    expect(splitToggleTarget(reading(cameras, '{"enabled":false,"slot":1}'), true, true)).toBe(true);
    expect(splitToggleTarget(reading(cameras, '{"enabled":true,"slot":1}'), true, true)).toBe(false);
    expect(splitToggleTarget(reading(cameras, '{"enabled":true,"slot":null}'), true, true)).toBeNull();
    expect(splitToggleTarget(null, true, true)).toBeNull();
    expect(splitToggleTarget(reading(cameras, '{"enabled":false,"slot":1}'), false, true)).toBeNull();
  });

  it('with video switched off neither the second camera nor the split button shows, even before the core catches up', () => {
    const stale = reading([camera(0, true), camera(1)], '{"enabled":true,"slot":1}');
    expect(pipCamera(stale, false)).toBeNull();
    expect(splitToggleTarget(stale, true, false)).toBeNull();
  });

  it('a swipe steps to the neighbouring camera and wraps around the ends', () => {
    const state = cameraSwitchState(reading([camera(0), camera(1, true), camera(2)]));
    expect(neighbourCamera(state, 1)?.slot).toBe(2);
    expect(neighbourCamera(state, -1)?.slot).toBe(0);
    expect(neighbourCamera(cameraSwitchState(reading([camera(0), camera(1), camera(2, true)])), 1)?.slot).toBe(0);
    expect(neighbourCamera(cameraSwitchState(reading([camera(0, true)])), 1)).toBeNull();
  });

  it('the chip names the camera on screen, or the first one when none is', () => {
    expect(chipCamera(reading([camera(0), camera(1, true)]))?.slot).toBe(1);
    expect(chipCamera(reading([camera(0), camera(1)]))?.slot).toBe(0);
    expect(chipCamera(reading([]))).toBeNull();
    expect(chipCamera(null)).toBeNull();
  });

  it("the pilot's pick shows at once and holds until the core sends a camera list made after it", () => {
    const shown = json(`{"class":"Cameras","cameras":[${camera(0, true)},${camera(1)}]}`);
    const picked = pickedReading(shown, { slot: 1, on: shown });
    expect(picked?.cameras.map((it) => it.active), 'the pick moves the active camera before the core answers').toEqual([false, true]);
    expect(pickedReading(shown, null)?.cameras.map((it) => it.active)).toEqual([true, false]);
    const answered = json(`{"class":"Cameras","cameras":[${camera(0, true)},${camera(1)}]}`);
    expect(pickedReading(answered, { slot: 1, on: shown })?.cameras.map((it) => it.active), "a list the core sent after the pick is the core's word, even when something else moved the camera back").toEqual([true, false]);
    expect(pickedReading(null, { slot: 1, on: null })).toBeNull();
  });

  it('the synthetic camera counts as chosen only when it is the one on screen', () => {
    const synthetic = `{"slot":1,"title":"Synthetic","short":"Synthetic","source":"${SYNTHETIC_SOURCE}","active":true,"status":"idle","problem":null}`;
    expect(syntheticChosen(reading([camera(0), synthetic]))).toBe(true);
    expect(syntheticChosen(reading([camera(0, true), synthetic.replace('"active":true', '"active":false')]))).toBe(false);
    expect(syntheticChosen(null)).toBe(false);
  });
});

import { describe, expect, it } from 'vitest';
import { CameraStatus, camerasReading } from '@/ui/Cameras';
import {
  SYNTHETIC_SOURCE,
  SyntheticDraw,
  dragStart,
  gimbalAim,
  pageOf,
  shownCameraStatus,
  shownNote,
  syntheticAims,
  syntheticAvailable,
  syntheticDrag,
  syntheticOverlaysScript,
  syntheticPan,
  syntheticPoseScript,
  syntheticTilePath,
  syntheticTilt,
  syntheticLook,
  verticalFov,
} from '@/ui/SyntheticView';

describe('SyntheticViewTest', () => {
  it('the synthetic view says which camera it stands in for once it has drawn, then steps off the aim point to the edge', () => {
    const standIn = 'No video from SITL camera · showing synthetic view';
    expect(shownNote(SyntheticDraw.Loading, standIn, true), 'an empty pane has no aim point to cover').toEqual({ text: 'Loading 3D view…', atEdge: false });
    expect(shownNote(SyntheticDraw.Failed, standIn, true)).toEqual({ text: '3D view failed to start', atEdge: false });
    expect(shownNote(SyntheticDraw.Drawn, standIn, false), 'the fallback is announced in the middle first').toEqual({ text: standIn, atEdge: false });
    expect(shownNote(SyntheticDraw.Drawn, standIn, true), 'once announced it leaves the middle, where the camera points, clear').toEqual({ text: standIn, atEdge: true });
    expect(shownNote(SyntheticDraw.Drawn, null, false), 'the synthetic view chosen as the camera says nothing').toBeNull();
  });

  it('the view is drawn only when the core can place the camera', () => {
    expect(syntheticAvailable({ available: true, latitude: 1.0 })).toBe(true);
    expect(syntheticAvailable({ available: false })).toBe(false);
    expect(syntheticAvailable(null)).toBe(false);
  });

  it('dragging the view down looks up and dragging up looks down, a full height being the vertical field of view', () => {
    expect(syntheticTilt(-15.0, -500, 1000, 40.0)).toBeCloseTo(-35.0, 9);
    expect(syntheticTilt(-15.0, 100, 1000, 40.0)).toBeCloseTo(-11.0, 9);
    expect(syntheticTilt(-15.0, 900, 1000, 40.0), 'it looks above the horizon').toBeCloseTo(21.0, 9);
    expect(syntheticTilt(-15.0, 2000, 1000, 40.0), 'as far up as the tilt slider reaches').toBeCloseTo(30.0, 9);
    expect(syntheticTilt(-15.0, -5000, 1000, 40.0)).toBeCloseTo(-90.0, 9);
  });

  it('dragging the view grabs it like a map: a drag right turns it left by the angle the drag spans', () => {
    expect(syntheticPan(0.0, 250, 1000, 70.0)).toBeCloseTo(-17.5, 9);
    expect(syntheticPan(0.0, -250, 1000, 70.0)).toBeCloseTo(17.5, 9);
    expect(syntheticPan(170.0, -200, 1000, 70.0)).toBeCloseTo(-176.0, 9);
    expect(verticalFov(90.0, 1000, 500)).toBeCloseTo(53.13, 2);
    expect(syntheticLook([-15.0, 0.0], { x: -250, y: 100 }, { width: 1000, height: 500 }, 90.0).map((it) => Number(it.toFixed(2)))).toEqual([-4.37, 22.5]);
  });

  it('the synthetic camera reads live only once the page has drawn it', () => {
    const camera = (source: string, status: string) =>
      camerasReading({ class: 'Cameras', cameras: [{ slot: 0, title: 'Synthetic', short: 'Synthetic', source, active: true, status }] })!.cameras[0]!;
    const synthetic = camera(SYNTHETIC_SOURCE, 'live');
    expect(shownCameraStatus(synthetic, true, SyntheticDraw.Loading), 'a green dot over a black pane told the pilot a blank view was working').toBe(CameraStatus.Connecting);
    expect(shownCameraStatus(synthetic, true, SyntheticDraw.Drawn)).toBe(CameraStatus.Live);
    expect(shownCameraStatus(synthetic, true, SyntheticDraw.Failed), 'a page that failed to start is no picture, not one on its way').toBe(CameraStatus.NoSignal);
    expect(shownCameraStatus(synthetic, false, SyntheticDraw.Loading), "a synthetic view that is not on screen has no page to wait for, so it keeps the core's word").toBe(CameraStatus.Live);
    expect(shownCameraStatus(camera(SYNTHETIC_SOURCE, 'idle'), true, SyntheticDraw.Failed)).toBe(CameraStatus.Idle);
    expect(shownCameraStatus(camera('RTSP Video Stream', 'live'), true, SyntheticDraw.Loading)).toBe(CameraStatus.Live);
  });

  it('with a gimbal aiming the view a finger drag steers the gimbal, without one it looks around', () => {
    const gimballed = { available: true, aimable: false, gimbalAimed: true };
    expect(syntheticDrag(gimballed, false, true), 'the view follows the gimbal, so a drag has to move the gimbal or it does nothing').toBe('gimbal');
    expect(syntheticDrag({ available: true, aimable: true, gimbalAimed: false }, false, true), 'free look keeps its own drag').toBe('look');
    expect(syntheticDrag(gimballed, false, false), 'a thumbnail or second pane never takes the drag').toBeNull();
    expect(syntheticDrag({ available: false, aimable: false }, false, true)).toBeNull();
    expect(syntheticDrag({ available: true, aimable: false, locksOnTarget: true }, false, true), 'a view locked on a tracked target is not the pilot\'s to aim').toBe('gimbal');
  });

  it('picking the synthetic view hands the drag to the view at once, before the core sends a view made after the pick', () => {
    const standingIn = { available: true, aimable: false, gimbalAimed: true, locksOnTarget: false };
    expect(syntheticDrag(standingIn, true, true), 'the gimbal belongs to the camera the pilot left, so the very next drag aims the view').toBe('look');
    const pickedEarlier = { available: true, aimable: true, gimbalAimed: true, locksOnTarget: false };
    expect(syntheticDrag(pickedEarlier, false, true), 'and switching back to the gimbal camera hands the drag back to the gimbal just as fast').toBe('gimbal');
    expect(syntheticDrag({ available: true, aimable: false, gimbalAimed: false, locksOnTarget: true }, true, true), 'a target lock still holds the picked view').toBe('gimbal');
  });

  it('a drag starts from where the active gimbal points', () => {
    const view = { gimbals: [{ active: false, pitch: -80.0, bodyYaw: 90.0 }, { active: true, pitch: -12.5, bodyYaw: 30.0 }] };
    expect(gimbalAim(view)).toEqual([-12.5, 30.0]);
    expect(gimbalAim({ gimbals: [{ active: true, pitch: null, bodyYaw: null }] }), 'a gimbal that has not reported its attitude gives no start').toBeNull();
    expect(gimbalAim(null)).toBeNull();
  });

  it('a second drag carries on from the first instead of snapping back to a stale report', () => {
    const reported = [-2.0, 0.0] as const;
    const sent = [-20.0, 75.0] as const;
    expect(dragStart(reported, sent, 300), 'the gimbal is still on its way to the last aim, so that is where the finger left it').toEqual(sent);
    expect(dragStart(reported, sent, 5000), "long after, the gimbal's own report wins, in case something else moved it").toEqual(reported);
    expect(dragStart(null, null, 0)).toEqual([0.0, 0.0]);
  });

  it('any page that has drawn wins, and a failure shows only when none has', () => {
    expect(pageOf([SyntheticDraw.Failed, SyntheticDraw.Drawn])).toBe(SyntheticDraw.Drawn);
    expect(pageOf([SyntheticDraw.Loading, SyntheticDraw.Failed])).toBe(SyntheticDraw.Failed);
    expect(pageOf([])).toBe(SyntheticDraw.Loading);
  });

  it('the page asks for tiles beside itself and the core answers them', () => {
    expect(syntheticTilePath('/assets/synthetic/tiles/Bing%20Satellite/15/29961/19829')).toBe('/Bing%20Satellite/15/29961/19829');
    expect(syntheticTilePath('/assets/synthetic/Cesium/Cesium.js')).toBeNull();
    expect(syntheticTilePath(null)).toBeNull();
  });

  it("the page gets the core's pose as it is, and only once it has loaded", () => {
    const script = syntheticPoseScript({ available: true, heading: 90 });
    expect(script.startsWith('window.aircast && window.aircast.pose(')).toBe(true);
    expect(script).toContain('"heading":90');
    expect(syntheticOverlaysScript({ route: [] }).startsWith('window.aircast && window.aircast.overlays(')).toBe(true);
  });

  it('sideways drags aim only a synthetic view picked as the camera, not one filling in for a dropped stream', () => {
    const aimable = { available: true, aimable: true };
    const cameras = (source: string) => camerasReading({ class: 'Cameras', cameras: [{ slot: 0, source, active: true }, { slot: 1, source: 'RTSP Video Stream', active: false }] });
    expect(syntheticAims(aimable, cameras(SYNTHETIC_SOURCE))).toBe(true);
    expect(syntheticAims(aimable, cameras('RTSP Video Stream')), 'a dropped RTSP stream shows the synthetic view as a fallback, and a swipe still switches cameras').toBe(false);
    expect(syntheticAims({ available: true, aimable: false, locksOnTarget: true }, cameras(SYNTHETIC_SOURCE)), 'a view locked on a tracked target takes no aim').toBe(false);
    expect(syntheticAims({ available: true, aimable: false, gimbalAimed: true }, cameras(SYNTHETIC_SOURCE)), 'picked a moment ago, it aims before the core sends a view that knows').toBe(true);
    expect(syntheticAims(aimable, null)).toBe(false);
  });
});

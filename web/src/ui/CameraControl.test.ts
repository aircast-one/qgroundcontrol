import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import {
  CAMERA_RECORD,
  CAMERA_STOP_PHOTO,
  CAM_MODE_PHOTO,
  type CameraReading,
  cameraDetails,
  cameraReading,
  lapsePlan,
  modeTapSwitches,
  shutterCaption,
  shutterReadout,
  zoomText,
} from './CameraControl';

const json = (text: string): Json => parseJson(text) ?? {};

const read = (view: Json | null): CameraReading => {
  const reading = cameraReading(view);
  if (reading === null) throw new Error('no camera present');
  return reading;
};

describe('CameraControlTest', () => {
  const camera = (text: string) => cameraReading(json(text));

  it('a camera with several streams names them and the one showing', () => {
    const reading = read(json('{"present":true,"streamLabels":["Wide","Narrow"],"currentStream":1}'));
    expect(reading.streamLabels).toEqual(['Wide', 'Narrow']);
    expect(reading.currentStream).toBe(1);
  });

  it('the panel offers the shutters the core serves, each with its readout', () => {
    const panel = read(
      json(`{"present":true,"panel":{"visible":true,"inPhotoMode":false,"selectVideoEnabled":true,"selectPhotoEnabled":false,"bothShown":true,
        "video":{"enabled":true,"capturing":true,"idle":false,"clock":"00:01:05"},
        "photo":{"enabled":true,"capturing":true,"idle":false,"press":null,"count":"00042"},
        "freeText":"Free: 12.0 GB","batteryText":null}}`),
    ).panel;
    if (panel === null) throw new Error('no panel');
    const [video, photo] = panel.shutters;
    if (video === undefined || photo === undefined) throw new Error('missing shutters');
    expect(video.action).toBe(CAMERA_RECORD);
    expect(video.recording && video.readoutActive).toBe(true);
    expect(video.readout).toBe('00:01:05');
    expect(photo.action).toBeNull();
    expect(photo.readout).toBe('00042');
    expect(shutterCaption(panel, video)).toBe('Video');
    expect(shutterReadout(video)).toBe('REC 00:01:05');
    expect(shutterReadout(photo)).toBe('00042');
    expect(shutterReadout({ ...video, readoutActive: false })).toBeNull();
    expect(panel.freeText).toBe('Free: 12.0 GB');
    expect(panel.batteryText).toBeNull();
    const lapse = read(json('{"present":true,"panel":{"visible":true,"bothShown":false,"photo":{"enabled":true,"capturing":true,"press":"stop","count":"00003"}}}')).panel;
    if (lapse === null) throw new Error('no panel');
    expect(lapse.shutters.length).toBe(1);
    const single = lapse.shutters[0];
    if (single === undefined) throw new Error('no shutter');
    expect(single.action).toBe(CAMERA_STOP_PHOTO);
    expect(shutterCaption(lapse, single)).toBeNull();
  });

  it('a survey camera can still be switched to photo, because survey is not photo', () => {
    const survey = read(json('{"present":true,"mode":2}'));
    expect(modeTapSwitches(survey, false)).toBe(true);
    expect(modeTapSwitches(survey, true)).toBe(true);
    expect(modeTapSwitches(read(json('{"present":true,"mode":0}')), false)).toBe(false);
  });

  it('no camera present is no controls', () => {
    expect(camera('{"present":true,"labels":["Sony","Thermal"],"selected":1}')?.selected).toBe(1);
    expect(camera('{"present":true,"labels":[],"selected":null}')?.selected).toBeNull();
    expect(cameraReading(null)).toBeNull();
    expect(cameraReading(json('{"present":false}'))).toBeNull();
  });
});

describe('CameraModeChangeTest', () => {
  const view = (extra: string) => json(`{"present":true,"hasModes":true,"modeText":"Photo",${extra}}`);

  it('a camera between operations may change mode', () => {
    expect(read(view('"canChangeMode":true')).canChangeMode).toBe(true);
  });

  it('a camera mid-operation may not change mode even though it has modes', () => {
    const reading = read(view('"canChangeMode":false'));
    expect(reading.hasModes).toBe(true);
    expect(reading.canChangeMode).toBe(false);
  });

  it('a view that never mentions canChangeMode does not invent permission', () => {
    expect(read(view('"mode":1')).canChangeMode).toBe(false);
  });
});

describe('CameraTimelapseTest', () => {
  const view = ({
    photoMode = '"timelapse"',
    canStopPhoto = false,
    lapseSeconds = '5.0',
    lapseCount = '10',
    lapseUnlimited = false,
    mode = CAM_MODE_PHOTO,
  }: { photoMode?: string; canStopPhoto?: boolean; lapseSeconds?: string; lapseCount?: string; lapseUnlimited?: boolean; mode?: number } = {}) =>
    json(`{"kind":"object","class":"Camera","present":true,"hasModes":true,"canChangeMode":true,
       "modeText":"Photo","isRecording":false,"canPhoto":true,"canRecord":true,
       "isTakingPhoto":false,"mode":${mode},"modeKnown":true,"photoMode":${photoMode},
       "canStopPhoto":${canStopPhoto},"lapseSeconds":${lapseSeconds},"lapseCount":${lapseCount},
       "lapseUnlimited":${lapseUnlimited}}`);

  it('a single-shot camera plans nothing and keeps its old button', () => {
    const single = read(view({ photoMode: '"single"', lapseSeconds: 'null', lapseCount: 'null' }));
    expect(lapsePlan(single)).toBeNull();
  });

  it('a shutter that starts ten shots does not say Take Photo', () => {
    expect(lapsePlan(read(view()))).toBe('every 5 s, 10 shots');
  });

  it('an unlimited lapse says it will not stop on its own', () => {
    expect(lapsePlan(read(view({ lapseCount: '0', lapseUnlimited: true })))).toBe('every 5 s, until stopped');
  });

  it('a whole-second interval is not written with a decimal', () => {
    expect(lapsePlan(read(view()))).toBe('every 5 s, 10 shots');
    expect(lapsePlan(read(view({ lapseSeconds: '2.5' })))).toBe('every 2.5 s, 10 shots');
  });
});

describe('CameraDetailsTest', () => {
  const view = ({
    reportsStorage = true,
    storageText = '"3.2 GB"',
    shotsText = '"00042"',
    batteryText = '"87%"',
    labels = '["SimCam","Thermal"]',
  }: { reportsStorage?: boolean; storageText?: string; shotsText?: string; batteryText?: string; labels?: string } = {}) =>
    json(`{"kind":"object","class":"CameraControl","present":true,"title":"SimCam",
       "labels":${labels},"stateText":"Idle","reportsStorage":${reportsStorage},
       "storageText":${storageText},"shotsText":${shotsText},"batteryText":${batteryText},
       "mode":0,"modeKnown":true,"canPhoto":true}`);

  it('the sheet lists what the camera actually reports', () => {
    expect(cameraDetails(read(view()))).toEqual([
      ['State', 'Idle'],
      ['Storage', '3.2 GB'],
      ['Photos', '00042'],
      ['Battery', '87%'],
    ]);
  });

  it('a camera that does not report storage gets no storage row rather than a blank one', () => {
    expect(cameraDetails(read(view({ reportsStorage: false }))).map(([label]) => label)).toEqual(['State', 'Photos', 'Battery']);
  });

  it('a camera with no battery reading drops that row too', () => {
    expect(cameraDetails(read(view({ batteryText: '""' }))).map(([label]) => label)).toEqual(['State', 'Storage', 'Photos']);
  });

  it('the labels come from the view, not from a second read of the manager', () => {
    expect(read(view()).labels).toEqual(['SimCam', 'Thermal']);
    expect(read(view({ labels: '[]' })).labels).toEqual([]);
  });
});

describe('CameraZoomTest', () => {
  const camera = (hasZoom = true, level = 50.0) =>
    read(
      json(`{"kind":"object","class":"CameraControl","present":true,"hasModes":true,
         "canChangeMode":true,"modeText":"Photo","isRecording":false,"canPhoto":true,
         "canRecord":true,"isTakingPhoto":false,"mode":0,"modeKnown":true,
         "hasZoom":${hasZoom},"zoomLevel":${level}}`),
    );

  it('a camera without zoom offers no control at all', () => {
    expect(zoomText(camera(false))).toBeNull();
  });

  it('the zoom chip states where the zoom is', () => {
    expect(zoomText(camera())).toBe('50%');
  });
});

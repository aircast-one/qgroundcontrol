import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { SettingsGroup, pageLook, settingsPages } from './SettingsScreen';
import { CameraKind, CameraStatus, VIDEO_SOURCES_PAGE, addableKinds, addedName, cameraDetail, camerasGlance, camerasReading, kindLabel, kindTitle } from './Cameras';
import { CameraDraft, CameraSave, NEW_CAMERA_DRAFT, cameraSave, editDraft, pickedKind, refusedDraft, tiltReadable } from './CamerasEditor';
import { SYNTHETIC_SOURCE } from './SyntheticView';

const json = (text: string): Json => parseJson(text) ?? {};

describe('CamerasTest', () => {
  const view = json(`
    {"kind":"object","class":"Cameras","readable":true,"reason":null,"active":2,
     "pip":{"enabled":false,"slot":0},
     "cameras":[
       {"slot":0,"stored":0,"title":"Front gimbal","short":"Front","name":"Front gimbal","source":"RTSP Video Stream","url":"rtsp://10.0.0.5:8554/front","problem":null,"fromDrone":false,"active":false,"status":"connecting"},
       {"slot":1,"stored":1,"title":"Camera 2","short":"Cam 2","name":"","source":"Back Camera","url":"","problem":"This kind of camera cannot show video in this app.","fromDrone":false,"active":false,"status":"bogus"},
       {"slot":2,"stored":null,"title":"SIYI A8","short":"SIYI","name":"SIYI A8","source":"UDP h.264 Video Stream","url":"0.0.0.0:5600","problem":null,"fromDrone":true,"active":true,"status":"live"}],
     "kinds":[
       {"raw":"Front Camera","label":"Front Camera","group":"This device","needsUrl":false,"hint":"","description":"This device's front camera"},
       {"raw":"RTSP Video Stream","label":"RTSP Video Stream","group":"Video streams","needsUrl":true,"hint":"rtsp://192.168.1.10:8554/live","description":"An rtsp:// address from an IP camera or video server"},
       {"raw":"UDP h.265 Video Stream","label":"UDP h.265 Video Stream","group":"Video streams","needsUrl":true,"hint":"0.0.0.0:5600","defaultAddress":"0.0.0.0:5600"},
       {"raw":"Herelink Hotspot","label":"Herelink Hotspot","group":"Vehicle and radio presets","needsUrl":false,"hint":"","description":"rtsp://192.168.43.1:8554/fpv_stream"},
       {"raw":"Back Camera","label":"Back Camera","group":"This device","needsUrl":false,"hint":""}]}
  `);

  const reading = () => {
    const parsed = camerasReading(view);
    if (parsed === null) throw new Error('the view did not read');
    return parsed;
  };

  it("every camera reads as one list, and only the operator's own can be edited", () => {
    const cameras = reading();
    expect(cameras.cameras.length).toBe(3);
    expect(cameras.stored.map((it) => it.stored)).toEqual([0, 1]);
    expect(cameras.cameras[2]?.stored).toBeNull();
    expect(cameras.cameras[2]?.fromDrone).toBe(true);
    expect(cameras.cameras[1]?.problem).toBe('This kind of camera cannot show video in this app.');
    expect(cameras.cameras[0]?.problem).toBeNull();
    expect(cameras.kinds[1]?.hint).toBe('rtsp://192.168.1.10:8554/live');
    expect(cameras.kinds[1]?.description).toBe('An rtsp:// address from an IP camera or video server');
    expect(cameras.kinds[2]?.defaultAddress).toBe('0.0.0.0:5600');
    expect(camerasReading(json('{"class":"Video"}'))).toBeNull();
  });

  it('each camera carries its signal, its short name, and the picture-in-picture pick', () => {
    const cameras = reading();
    expect(cameras.cameras.map((it) => it.status)).toEqual([CameraStatus.Connecting, CameraStatus.Idle, CameraStatus.Live]);
    expect(cameras.cameras.map((it) => it.short)).toEqual(['Front', 'Cam 2', 'SIYI']);
    expect(cameras.pip).toEqual({ enabled: false, slot: 0 });
    expect(camerasReading(json('{"class":"Cameras","pip":{"enabled":true,"slot":null}}'))?.pip).toEqual({ enabled: true, slot: null });
    expect(camerasReading(json('{"class":"Cameras","cameras":[{"status":"noSignal"}]}'))?.cameras.map((it) => it.status)).toEqual([CameraStatus.NoSignal]);
  });

  it('a row names its type beside an address that does not spell it, or that the drone brought it', () => {
    const cameras = reading().cameras;
    expect(cameras.map(cameraDetail)).toEqual(['rtsp://10.0.0.5:8554/front', 'Back camera', 'From the drone']);
    const front = cameras[0];
    if (front === undefined) throw new Error('no front camera');
    expect(cameraDetail({ ...front, source: 'UDP h.265 Video Stream', url: '0.0.0.0:5600' })).toBe('UDP h.265 · 0.0.0.0:5600');
  });

  it('the type list puts streams first, then presets, then this phone, never a fixed camera already listed', () => {
    const cameras = reading();
    expect(addableKinds(cameras).map((it) => it.raw)).toEqual(['RTSP Video Stream', 'UDP h.265 Video Stream', 'Herelink Hotspot', 'Front Camera']);
    expect(addableKinds(cameras, 'Back Camera').some((it) => it.raw === 'Back Camera')).toBe(true);
    expect(addableKinds(cameras).map(kindTitle)).toEqual(['RTSP', 'UDP h.265', 'Herelink Hotspot', 'Front camera']);
    expect(addableKinds(null)).toEqual([]);
  });

  it('the synthetic view is offered by its own name', () => {
    const synthetic = CameraKind({ raw: SYNTHETIC_SOURCE, label: SYNTHETIC_SOURCE, group: 'This device', needsUrl: false, hint: '' });
    expect(kindTitle(synthetic)).toBe('Synthetic view');
    expect(addedName(synthetic)).toBe('Synthetic view');
    expect(addedName({ ...synthetic, raw: 'Front Camera' })).toBe('');
  });

  const udp = CameraKind({ raw: 'UDP h.265 Video Stream', label: 'UDP h.265 Video Stream', group: 'Video streams', needsUrl: true, hint: '0.0.0.0:5600', defaultAddress: '0.0.0.0:5600' });
  const rtsp = CameraKind({ raw: 'RTSP Video Stream', label: 'RTSP Video Stream', group: 'Video streams', needsUrl: true, hint: 'rtsp://192.168.1.10:8554/live' });
  const hotspot = CameraKind({ raw: 'Herelink Hotspot', label: 'Herelink Hotspot', group: 'Vehicle and radio presets', needsUrl: false, hint: '' });

  it('picking a type starts the address where most radios send, and keeps one already typed', () => {
    const udpDraft = pickedKind(NEW_CAMERA_DRAFT, udp);
    expect(udpDraft).toEqual(CameraDraft({ stored: null, title: '', name: '', source: udp.raw, url: '0.0.0.0:5600' }));
    expect(pickedKind(NEW_CAMERA_DRAFT, rtsp).url).toBe('');
    expect(pickedKind({ ...udpDraft, choosingKind: true }, rtsp).url).toBe('0.0.0.0:5600');
    const preset = pickedKind(udpDraft, hotspot);
    expect([preset.needsUrl, preset.url]).toEqual([false, '']);
  });

  it('a new camera is added with the type picked, an edit updates its slot', () => {
    expect(cameraSave({ ...pickedKind(NEW_CAMERA_DRAFT, udp), name: 'Belly' })).toEqual(CameraSave.Add('Belly', udp.raw, '0.0.0.0:5600', null));
    const front = reading().cameras[0];
    if (front === undefined) throw new Error('no front camera');
    const opened = editDraft(front, reading().kinds);
    if (opened === null) throw new Error('the front camera did not open');
    const edited = { ...pickedKind(opened, udp), url: '0.0.0.0:5601' };
    expect(cameraSave(edited)).toEqual(CameraSave.Update(0, 'Front gimbal', udp.raw, '0.0.0.0:5601', null));
    const drone = reading().cameras[2];
    if (drone === undefined) throw new Error('no drone camera');
    expect(editDraft(drone, [])).toBeNull();
  });

  it('a camera fixed to the frame is saved with its tilt, and an empty tilt leaves it to the gimbal', () => {
    const front = reading().cameras[0];
    if (front === undefined) throw new Error('no front camera');
    expect(front.tilt).toBeNull();
    const fixed = { ...front, tilt: -30.0 };
    expect(editDraft(fixed, [])?.tilt).toBe('-30');
    const plain = editDraft(front, []);
    const fixedDraft = editDraft(fixed, []);
    if (plain === null || fixedDraft === null) throw new Error('the camera did not open');
    expect(cameraSave({ ...plain, tilt: ' -12,5 ' })).toEqual(CameraSave.Update(0, 'Front gimbal', front.source, front.url, -12.5));
    expect(cameraSave({ ...fixedDraft, tilt: '' })).toEqual(CameraSave.Update(0, 'Front gimbal', front.source, front.url, null));
    expect(tiltReadable('')).toBe(true);
    expect(tiltReadable('down')).toBe(false);
    const tilted = camerasReading(json(JSON.stringify(view).replace('"url":"rtsp://10.0.0.5:8554/front",', '"url":"rtsp://10.0.0.5:8554/front","tilt":-90,')));
    expect(tilted?.cameras[0]?.tilt).toBe(-90.0);
  });

  it('a refused save lands in the sheet as it is now, and never reopens a sheet the operator closed', () => {
    const typedWhileSaving = { ...pickedKind(NEW_CAMERA_DRAFT, udp), name: 'Belly cam' };
    expect(refusedDraft(typedWhileSaving, 'No')).toEqual({ ...typedWhileSaving, refusal: 'No' });
    expect(refusedDraft(null, 'No')).toBeNull();
    expect(refusedDraft(typedWhileSaving, null)).toBeNull();
  });

  it('the video sources page is listed under Transmission though it has no settings of its own', () => {
    const pages = settingsPages(json('{"pages":[{"title":"Video sources","showsVideoSources":true,"sections":[]}]}'));
    expect(pages.map((it) => it.title)).toEqual([VIDEO_SOURCES_PAGE]);
    expect(pageLook(VIDEO_SOURCES_PAGE).group).toBe(SettingsGroup.Transmission);
  });

  it('the settings index names the camera on screen and how many there are', () => {
    expect(camerasGlance(camerasReading(view))).toBe('SIYI A8 · 3 cameras');
    expect(camerasGlance(null)).toBe('');
    expect(kindLabel('RTSP Video Stream')).toBe('RTSP');
  });
});

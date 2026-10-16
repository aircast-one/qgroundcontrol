import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { CameraSettingControl, cameraSettings } from './CameraSettingsSection';

const json = (text: string): Json => parseJson(text) ?? {};

describe('CameraSettingsSectionTest', () => {
  const view = json(`{"state":"ready","activeSettings":["CAM_EV","CAM_WBMODE","CAM_FLICKER","CAM_NAME"],"parameters":[
      {"name":"CAM_WBMODE","description":"White Balance","readOnly":false,"isBool":false,"value":1,"selected":1,"options":[{"label":"Auto","value":0},{"label":"Sunny","value":1}]},
      {"name":"CAM_EV","description":"Exposure Compensation","readOnly":false,"isBool":false,"value":0.5,"min":-2.0,"max":2.0,"step":0.5,"options":[]},
      {"name":"CAM_FLICKER","description":"Flicker","readOnly":true,"isBool":true,"value":1,"options":[]},
      {"name":"CAM_NAME","description":"Name","readOnly":false,"isBool":false,"value":"E90","options":[]},
      {"name":"CAM_HIDDEN","description":"Hidden","readOnly":false,"isBool":false,"value":0,"options":[]}]}`);

  it("only the active settings show, in the camera's order, each with qgc's control", () => {
    const shown = cameraSettings(view);
    expect(shown.map((it) => it.name)).toEqual(['CAM_EV', 'CAM_WBMODE', 'CAM_FLICKER', 'CAM_NAME']);
    expect(shown[0]?.control).toEqual(CameraSettingControl.Range(-2.0, 2.0, 0.5, 0.5));
    const choice = shown[1]?.control;
    expect(choice?.kind === 'Choice' ? choice.selected : null).toBe(1);
    expect(shown[2]?.control).toEqual(CameraSettingControl.Toggle(true));
    expect(shown[2]?.readOnly).toBe(true);
    expect(shown[3]?.control).toEqual(CameraSettingControl.Entry('E90'));
  });

  it('nothing shows until the definition is loaded', () => {
    expect(cameraSettings(json('{"state":"fetching"}'))).toEqual([]);
    expect(cameraSettings(null)).toEqual([]);
  });
});

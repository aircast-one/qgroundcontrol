import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { offeredActions } from './MapClickMenu';
import { PICTURE_HOLD_HINT, PICTURE_RIGHT_CLICK_HINT, pictureHoldHintText, picturePress } from './PictureHold';

const json = (text: string): Json => parseJson(text) ?? {};

describe('PictureHoldTest', () => {
  const spot = { x: 0.5, y: 0.5 };

  it('a point on the ground opens the actions the core lists for the picture', () => {
    const press = picturePress(spot, json('{"ok":true,"latitude":-35.0001,"longitude":149.0002,"actions":["GoTo","Roi"]}'));
    expect(press.point).toEqual({ latitude: -35.0001, longitude: 149.0002 });
    expect(press.actions).toEqual(['GoTo', 'Roi']);
    expect(press.spot).toBe(spot);
  });

  it('the sky carries the reason instead of a point', () => {
    const press = picturePress(spot, json('{"ok":false,"refusal":"notGround","reason":"That is above the horizon, or farther than 2000 m."}'));
    expect(press.point).toBeNull();
    expect(press.reason).toBe('That is above the horizon, or farther than 2000 m.');
    expect(picturePress(spot, null).point).toBeNull();
  });

  it('the picture menu keeps only the actions offered for the picture, in the map order', () => {
    const view = json('{"actions":[{"id":"GoTo"},{"id":"Orbit"},{"id":"Roi"},{"id":"SetHome"},{"id":"Heading"}]}');
    expect(offeredActions(view, ['GoTo', 'Orbit', 'Roi', 'Heading']).map((it) => it.id)).toEqual(['GoTo', 'Orbit', 'Roi', 'Heading']);
    expect(offeredActions(view, null).map((it) => it.id)).toContain('SetHome');
  });

  it('the hint names the gesture of the device', () => {
    expect(pictureHoldHintText(true)).toBe(PICTURE_RIGHT_CLICK_HINT);
    expect(pictureHoldHintText(false)).toBe(PICTURE_HOLD_HINT);
  });
});

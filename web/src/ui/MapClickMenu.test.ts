import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { MINIMUM_CIRCLE_RADIUS_METRES } from '@/map/MissionLayers';
import {
  coordinateLines,
  goHereText,
  gotoPreview,
  gotoPreviewLines,
  gotoPreviewPath,
  loiterEditOpened,
  loiterOffer,
  loiterRadiusField,
  loiterTyped,
  mapClickActions,
  orbitArgs,
  orbitDefaults,
  orbitEdit,
  orbitOpened,
  radiusMetres,
  setWaypointMessage,
  signedLoiterRadius,
  waypointTarget,
} from './MapClickMenu';

const json = (text: string): Json => parseJson(text) ?? {};

describe('MapClickMenuTest', () => {
  it('reads the offered actions in the order the core serves them', () => {
    const actions = mapClickActions(
      json(
        '{"actions":[{"id":"GoTo","path":"vehicle.guidedModeGotoLocation","label":"Go to location","title":"Go To Location",' +
          '"message":"Move the vehicle to the specified location","confirm":true},' +
          '{"path":"vehicle.guidedModeROI","label":"ROI at location","title":"ROI","message":"m","confirm":false}]}',
      ),
    );
    expect(actions.map((it) => it.label)).toEqual(['Go to location', 'ROI at location']);
    expect(actions[0]!.confirm).toBe(true);
    expect(actions[1]!.confirm).toBe(false);
    expect(mapClickActions(null)).toHaveLength(0);
  });

  it('an orbit sends the point, radius, direction and height above home in that order', () => {
    const args = orbitArgs({ latitude: 47.4, longitude: 8.5 }, { radiusMetres: 30.0, clockwise: false, aboveHomeMetres: 50.0 });
    expect(args).toEqual([47.4, 8.5, 30.0, false, 50.0]);
    const feet = orbitDefaults(json('{"orbitDefaultRadius":98.4252,"orbitRadiusUnit":"ft","orbitMetresPerUnit":0.3048,"orbitClockwise":true}'));
    expect(radiusMetres('', feet)!).toBeCloseTo(30.0, 3);
    expect(radiusMetres('50', feet)!).toBeCloseTo(15.24, 9);
    expect(radiusMetres('wide', feet)).toBeNull();
  });

  it('orbit at location opens a default circle on the tapped point that the radius field edits in app units', () => {
    const feet = orbitDefaults(json('{"orbitDefaultRadius":98.4252,"orbitRadiusUnit":"ft","orbitMetresPerUnit":0.3048,"orbitClockwise":true}'));
    const opened = orbitOpened({ latitude: 47.4, longitude: 8.5 }, feet);
    expect(opened.centre).toEqual({ latitude: 47.4, longitude: 8.5 });
    expect(opened.radiusMetres).toBeCloseTo(30.0, 3);
    expect(opened.clockwise).toBe(true);
    const edit = orbitEdit({ ...opened, radiusMetres: 15.24, clockwise: false }, feet);
    expect(loiterRadiusField(null, edit)).toBe('50');
    expect(loiterTyped('100', edit).radiusMetres).toBeCloseTo(30.48, 9);
    expect(orbitOpened({ latitude: 47.4, longitude: 8.5 }, orbitDefaults(null)).radiusMetres).toBe(MINIMUM_CIRCLE_RADIUS_METRES);
  });

  it('shows the point to six places as QGC does', () => {
    expect(coordinateLines({ latitude: 47.397, longitude: 8.5451234 })).toEqual(['Lat: 47.397000', 'Lon: 8.545123']);
  });

  it('a tapped item jumps to its sequence, never before the first waypoint', () => {
    expect(waypointTarget(0)).toBe(1);
    expect(waypointTarget(4)).toBe(4);
    expect(setWaypointMessage(4)).toBe('Adjust current waypoint to 4');
  });

  it('a loiter change re-sends the goto point with the direction as the sign', () => {
    const offer = loiterOffer(json('{"loiter":{"latitude":47.4,"longitude":8.5,"title":"Change Loiter Radius","message":"m","defaultRadius":80,"clockwise":true}}'))!;
    expect(offer.latitude).toBe(47.4);
    expect(offer.defaultRadius).toBe(80.0);
    expect(signedLoiterRadius(80.0, false)).toBe(-80.0);
    expect(signedLoiterRadius(-80.0, true)).toBe(80.0);
    expect(loiterOffer(json('{"loiter":null}'))).toBeNull();
  });

  it('the loiter radius field follows the map drag but keeps what the operator is typing', () => {
    const offer = loiterOffer(json('{"loiter":{"latitude":47.4,"longitude":8.5,"title":"t","message":"m","defaultRadius":250,"clockwise":false}}'))!;
    const opened = loiterEditOpened(offer, { radius: 0.0, unit: 'ft', metresPerUnit: 0.3048, clockwise: true });
    expect(opened.radiusMetres).toBeCloseTo(76.2, 9);
    expect(opened.clockwise).toBe(false);
    expect(loiterRadiusField(null, opened)).toBe('250');
    expect(loiterRadiusField('250.', opened)).toBe('250.');
    expect(loiterRadiusField('', opened)).toBe('');
    const dragged = { ...opened, radiusMetres: 100 * 0.3048 };
    expect(loiterRadiusField('250', dragged)).toBe('100');
    expect(loiterTyped('100', opened).radiusMetres).toBeCloseTo(30.48, 9);
    expect(loiterTyped('abc', opened)).toEqual(opened);
    expect(loiterTyped('0', opened)).toEqual(opened);
  });

  it("go here reads how far and which way the point lies, in the operator's unit", () => {
    const from = { latitude: 47.0, longitude: 8.0 };
    expect(goHereText(from, { latitude: 47.001, longitude: 8.0 }, 'm', 1.0)).toBe('111 m north');
    expect(goHereText(from, { latitude: 47.001, longitude: 8.0 }, 'ft', 0.3048)).toBe('365 ft north');
    expect(goHereText(from, { latitude: 47.0, longitude: 8.001 }, 'm', 1.0)).toBe('76 m east');
    expect(goHereText(from, { latitude: 47.001, longitude: 8.0 }, '', 1.0)).toBeNull();
  });

  it('the go-to preview names height and battery, and warns past the return point', () => {
    const view = json('{"available":true,"distanceText":"81 m","heightText":"at 20 m (current height)","verdict":"Past your return point. Return home instead?","pastReturnPoint":true}');
    const preview = gotoPreview(view)!;
    expect(preview.pastReturnPoint).toBe(true);
    expect(gotoPreviewLines({ ...preview, distance: '81 m north-west' }, 'Move the vehicle')).toEqual(['81 m north-west · at 20 m (current height)', 'Past your return point. Return home instead?']);
    expect(gotoPreviewLines(null, 'Move the vehicle')).toEqual(['Move the vehicle']);
    expect(gotoPreviewPath({ latitude: 47.398, longitude: 8.546 })).toBe('view.gotoPreview(47.3980000,8.5460000)');
  });
});

import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import {
  TrafficAlert,
  TrafficContact,
  TrafficLevel,
  TrafficUnits,
  compassPoint,
  trafficAlert,
  trafficCaption,
  trafficContactText,
  trafficContactUrgent,
  trafficEmergencyText,
  trafficHeightText,
  trafficLevel,
  trafficReading,
  trafficShown,
  trafficSummary,
} from './TrafficView';

const json = (text: string): Json => parseJson(text) ?? {};

describe('TrafficViewTest', () => {
  const near = `{"icaoAddress":11259375,"callsign":"BAW42","distance":1200.0,
        "bearingDegrees":95.0,"altitude":450.0,"relativeAltitude":200.0,"emergency":null,
        "alert":false,"stale":false}`;

  const synthetic = (alert: boolean = false) => `${near.replace('"alert":false', `"alert":${alert}`).slice(0, -1)},"simulated":true}`;

  const view = ({
    enabled = true,
    available = true,
    connected = true,
    receiving = true,
    ownPositionKnown = true,
    alerting = 'null',
    alertUnknown = 0,
    emergency = 'null',
    error = 'null',
    contacts = '[]',
  }: {
    enabled?: boolean;
    available?: boolean;
    connected?: boolean;
    receiving?: boolean;
    ownPositionKnown?: boolean;
    alerting?: string;
    alertUnknown?: number;
    emergency?: string;
    error?: string;
    contacts?: string;
  } = {}) =>
    json(`{"kind":"object","class":"AdsbTraffic","enabled":${enabled},"available":${available},
       "connected":${connected},"receiving":${receiving},"ownPositionKnown":${ownPositionKnown},"alerting":${alerting},"alertUnknown":${alertUnknown},
       "emergency":${emergency},"error":${error},
       "units":{"altitude":"m","velocity":"m/s","heading":"deg","distance":"m","bearing":"deg"},
       "contacts":${contacts}}`);

  it('a sky with nothing but synthetic targets does not report aircraft', () => {
    expect(trafficSummary(trafficReading(view({ contacts: `[${synthetic()},${synthetic()}]` }))!)).toBe('Traffic: 2 simulated');
  });

  it('real and synthetic targets are counted apart', () => {
    expect(trafficSummary(trafficReading(view({ contacts: `[${near},${synthetic()},${synthetic()}]` }))!)).toBe('Traffic: 1 aircraft · 2 simulated');
  });

  it('a sky of real aircraft reads exactly as before', () => {
    expect(trafficSummary(trafficReading(view({ contacts: `[${near}]` }))!)).toBe('Traffic: 1 aircraft');
    expect(trafficSummary(trafficReading(view({ contacts: `[${near},${near},${near}]` }))!)).toBe('Traffic: 3 aircraft');
  });

  it('a simulated target is not an aircraft and the row says so first', () => {
    const reading = trafficReading(view({ contacts: `[${synthetic(true)}]` }))!;
    const line = trafficContactText(reading.contacts[0]!, reading.units);
    expect(line.startsWith('simulated')).toBe(true);
    expect(line).toContain('alerting');
  });

  it('a real contact carries no such mark', () => {
    const reading = trafficReading(view({ contacts: `[${near}]` }))!;
    expect(trafficContactText(reading.contacts[0]!, reading.units)).not.toContain('simulated');
  });

  it('a payload from another view is not a traffic reading', () => {
    expect(trafficReading(null)).toBeNull();
    expect(trafficReading(json('{"kind":"object","class":"Detections"}'))).toBeNull();
  });

  it("a contact carries the core's converted numbers and the block carries the unit", () => {
    const reading = trafficReading(view({ contacts: `[${near}]` }))!;
    expect(reading.contacts.length).toBe(1);
    expect(reading.contacts[0]!.name).toBe('BAW42');
    expect(trafficContactText(reading.contacts[0]!, reading.units)).toBe('1200 m  95 deg  200 m above');
  });

  it('a contact with no callsign is named by its ICAO address', () => {
    const anonymous = near.replace('"callsign":"BAW42"', '"callsign":null');
    const reading = trafficReading(view({ contacts: `[${anonymous}]` }))!;
    expect(reading.contacts[0]!.name).toBe('ABCDEF');
  });

  it('the core withholds range when it cannot place the operator, and zero would be a lie', () => {
    const unplaced = near.replace('"distance":1200.0', '"distance":null').replace('"bearingDegrees":95.0', '"bearingDegrees":null');
    const reading = trafficReading(view({ contacts: `[${unplaced}]` }))!;
    expect(trafficContactText(reading.contacts[0]!, reading.units)).toBe('bearing unknown  200 m above');
  });

  it('a stale fix says so, because at jet speed it is not where it is drawn', () => {
    const old = near.replace('"stale":false', '"stale":true');
    const reading = trafficReading(view({ contacts: `[${old}]` }))!;
    expect(trafficContactText(reading.contacts[0]!, reading.units)).toBe('1200 m  95 deg  200 m above  stale');
  });

  it('the server switch does not govern what the vehicle relays over MAVLink', () => {
    expect(trafficShown(trafficReading(view({ enabled: false }))!)).toBe(false);
    expect(trafficShown(trafficReading(view({ enabled: false, contacts: `[${near}]` }))!)).toBe(true);
    expect(trafficShown(trafficReading(view())!)).toBe(true);
  });

  it('a configured receiver switched off is not an absent one', () => {
    expect(trafficSummary(trafficReading(view({ available: false, receiving: false }))!)).toBe('No traffic receiver');
    expect(trafficReading(view({ enabled: false }))!.enabled).toBe(false);
  });

  it('each way of hearing nothing gets its own sentence', () => {
    expect(trafficSummary(trafficReading(view())!)).toBe('Traffic clear');
    expect(trafficSummary(trafficReading(view({ receiving: false }))!)).toBe('No traffic feed');
    expect(trafficSummary(trafficReading(view({ receiving: false, error: '{"token":"connectFailed","detail":"connection refused"}' }))!)).toBe('Traffic server unreachable');
    expect(trafficSummary(trafficReading(view({ receiving: false, error: '{"token":"linkLost","detail":"closed"}' }))!)).toBe('Traffic feed dropped');
  });

  it('contacts outrank every complaint about the feed, because they are the answer asked for', () => {
    const relayed = view({ available: false, receiving: false, contacts: `[${near}]` });
    expect(trafficSummary(trafficReading(relayed)!)).toBe('Traffic: 1 aircraft');
  });

  it('an unreported alert is caution, not a calm sky', () => {
    expect(trafficLevel(trafficReading(view())!)).toBe(TrafficLevel.Good);
    expect(trafficLevel(trafficReading(view({ alertUnknown: 1 }))!)).toBe(TrafficLevel.Caution);
    expect(trafficLevel(trafficReading(view({ connected: false, receiving: false }))!)).toBe(TrafficLevel.Caution);
    expect(trafficLevel(trafficReading(view({ alerting: 'true' }))!)).toBe(TrafficLevel.Warning);
  });

  it('relayed traffic never states an alert, so its unknown count is not a signal', () => {
    const relayed = view({ available: false, connected: false, receiving: true, alertUnknown: 1, contacts: `[${near}]` });
    expect(trafficLevel(trafficReading(relayed)!)).toBe(TrafficLevel.Good);
  });

  it('an aircraft squawking an emergency outranks a merely close one', () => {
    const panic = view({ alerting: 'true', emergency: '"hijack"', contacts: `[${near}]` });
    expect(trafficLevel(trafficReading(panic)!)).toBe(TrafficLevel.Critical);
    expect(trafficEmergencyText(trafficReading(panic)!.emergency)).toBe('squawking hijack');
  });

  it('a stated all-clear is not an unknown one', () => {
    expect(trafficReading(view())!.alerting).toBeNull();
    expect(trafficReading(view({ alerting: 'false' }))!.alerting).toBe(false);
  });

  it('a coarse unit keeps a decimal, or a close aircraft in miles reads as zero away', () => {
    const miles = json(`{"kind":"object","class":"AdsbTraffic","enabled":true,"available":true,"receiving":true,
       "alerting":null,"alertUnknown":0,"emergency":null,"error":null,
       "units":{"altitude":"ft","velocity":"mph","heading":"deg","distance":"mi","bearing":"deg"},
       "contacts":[{"icaoAddress":1,"callsign":"N1","distance":0.4,"bearingDegrees":95.0,
       "altitude":1500.0,"relativeAltitude":200.0,"emergency":null,"alert":false,"stale":false}]}`);
    const reading = trafficReading(miles)!;
    expect(trafficContactText(reading.contacts[0]!, reading.units)).toBe('0.4 mi  95 deg  200 ft above');
  });

  it('height is stated against my own, because at my level is the question being asked', () => {
    const at = (relative: string) => trafficReading(view({ contacts: `[${near.replace('"relativeAltitude":200.0', `"relativeAltitude":${relative}`)}]` }))!;
    expect(trafficContactText(at('200.0').contacts[0]!, at('200.0').units)).toBe('1200 m  95 deg  200 m above');
    expect(trafficContactText(at('-150.0').contacts[0]!, at('-150.0').units)).toBe('1200 m  95 deg  150 m below');
    expect(trafficContactText(at('0.0').contacts[0]!, at('0.0').units)).toBe('1200 m  95 deg  my level');
    expect(trafficContactText(at('null').contacts[0]!, at('null').units)).toBe('1200 m  95 deg  450 m');
  });

  it('the contact the block is warning about says so on its own row', () => {
    const alerting = near.replace('"alert":false', '"alert":true');
    const reading = trafficReading(view({ alerting: 'true', contacts: `[${alerting}]` }))!;
    expect(trafficContactText(reading.contacts[0]!, reading.units)).toBe('1200 m  95 deg  200 m above  alerting');
  });

  it('the reference for every number is stated once, not guessed from a row', () => {
    expect(trafficCaption(trafficReading(view())!)).toBe('Range, bearing and height are relative to the vehicle');
    expect(trafficCaption(trafficReading(view({ ownPositionKnown: false }))!)).toBe('No vehicle position, so nothing can be ranged');
  });

  it('the row that earned the warning is the one marked, not the whole list', () => {
    const calm = trafficReading(view({ contacts: `[${near}]` }))!.contacts[0]!;
    const loud = trafficReading(view({ contacts: `[${near.replace('"alert":false', '"alert":true')}]` }))!.contacts[0]!;
    const squawking = trafficReading(view({ contacts: `[${near.replace('"emergency":null', '"emergency":"hijack"')}]` }))!.contacts[0]!;
    expect(trafficContactUrgent(calm)).toBe(false);
    expect(trafficContactUrgent(loud)).toBe(true);
    expect(trafficContactUrgent(squawking)).toBe(true);
  });

  it('an absolute altitude says what it is measured from', () => {
    const units = TrafficUnits({ distance: 'ft', altitude: 'ft', heading: 'deg' });
    const contact = (type: string) =>
      TrafficContact({ icaoAddress: 1, callsign: 'SWR000', distance: null, bearingDegrees: null, altitude: 1000.0, relativeAltitude: null, emergency: '', alert: null, stale: false, altitudeType: type });
    expect(trafficHeightText(contact('pressureQnh'), units)).toBe('1000 ft by pressure');
    expect(trafficHeightText(contact('geometric'), units)).toBe('1000 ft by GPS');
    expect(trafficHeightText(contact('somethingNew'), units), 'a datum this head does not know is left unnamed rather than guessed').toBe('1000 ft');
  });

  it('a height relative to the vehicle still reads as a separation', () => {
    const units = TrafficUnits({ distance: 'ft', altitude: 'ft', heading: 'deg' });
    const above = TrafficContact({ icaoAddress: 1, callsign: 'A', distance: 100.0, bearingDegrees: 90.0, altitude: 1000.0, relativeAltitude: 500.0, emergency: '', alert: null, stale: false, altitudeType: 'pressureQnh' });
    expect(trafficHeightText(above, units)).toBe('500 ft above');
  });

  it('a calm sky raises no banner, however much traffic is drawn', () => {
    expect(trafficAlert(trafficReading(view({ contacts: `[${near},${synthetic()}]` }))!)).toBeNull();
    expect(trafficAlert(trafficReading(view({ enabled: false }))!)).toBeNull();
  });

  it('a feed problem is a caution that names the problem, not a count of aircraft', () => {
    expect(trafficAlert(trafficReading(view({ receiving: false, error: '{"token":"linkLost","detail":"closed"}', contacts: `[${near}]` }))!)).toEqual(
      TrafficAlert(TrafficLevel.Caution, 'Traffic feed dropped', ''),
    );
    expect(trafficAlert(trafficReading(view({ receiving: false }))!)).toEqual(TrafficAlert(TrafficLevel.Caution, 'No traffic feed', ''));
    expect(trafficAlert(trafficReading(view({ alertUnknown: 2 }))!)).toEqual(TrafficAlert(TrafficLevel.Caution, 'Collision alerts unknown for 2 aircraft', ''));
  });

  it('a warning names the closest alerting aircraft, where it is and how high', () => {
    const far = near.replace('BAW42', 'EZY9').replace('1200.0', '5000.0').replace('"alert":false', '"alert":true');
    const close = near.replace('"alert":false', '"alert":true');
    expect(trafficAlert(trafficReading(view({ alerting: 'true', contacts: `[${far},${near},${close}]` }))!)).toEqual(
      TrafficAlert(TrafficLevel.Warning, 'Aircraft nearby', 'BAW42 · 1200 m E · 200 m above'),
    );
    expect(trafficAlert(trafficReading(view({ alerting: 'true', emergency: '"hijack"', contacts: `[${near}]` }))!)).toEqual(TrafficAlert(TrafficLevel.Critical, 'Aircraft squawking hijack', ''));
  });

  it('a bearing reads as the nearest of eight compass points', () => {
    expect([0.0, 44.0, 95.0, 180.0, 315.0, 350.0, -10.0].map(compassPoint)).toEqual(['N', 'NE', 'E', 'S', 'NW', 'N', 'N']);
  });
});

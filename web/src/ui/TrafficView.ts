import { type Json, isNull, jsonObjects, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { format } from '@/kotlin/Std';

export const TRAFFIC_VIEW = 'view.adsbTraffic';

export type TrafficLevel = { readonly name: 'Good' | 'Caution' | 'Warning' | 'Critical'; readonly ordinal: number };

export const TrafficLevel = {
  Good: { name: 'Good', ordinal: 0 },
  Caution: { name: 'Caution', ordinal: 1 },
  Warning: { name: 'Warning', ordinal: 2 },
  Critical: { name: 'Critical', ordinal: 3 },
} as const satisfies Record<string, TrafficLevel>;

export const TrafficLevelEntries: readonly TrafficLevel[] = [TrafficLevel.Good, TrafficLevel.Caution, TrafficLevel.Warning, TrafficLevel.Critical];

export type TrafficContact = {
  readonly icaoAddress: number;
  readonly callsign: string;
  readonly distance: number | null;
  readonly bearingDegrees: number | null;
  readonly altitude: number | null;
  readonly relativeAltitude: number | null;
  readonly emergency: string;
  readonly alert: boolean | null;
  readonly stale: boolean;
  readonly altitudeType: string;
  readonly simulated: boolean;
  readonly name: string;
  readonly located: boolean;
};

export const TrafficContact = (fields: {
  icaoAddress: number;
  callsign: string;
  distance: number | null;
  bearingDegrees: number | null;
  altitude: number | null;
  relativeAltitude: number | null;
  emergency: string;
  alert: boolean | null;
  stale: boolean;
  altitudeType?: string;
  simulated?: boolean;
}): TrafficContact => ({
  altitudeType: '',
  simulated: false,
  ...fields,
  name: fields.callsign.trim() === '' ? format('%06X', fields.icaoAddress) : fields.callsign,
  located: fields.distance !== null && fields.bearingDegrees !== null,
});

export type TrafficUnits = { distance: string; altitude: string; heading: string };

export const TrafficUnits = (fields: TrafficUnits): TrafficUnits => ({ ...fields });

export type TrafficReading = {
  enabled: boolean;
  available: boolean;
  connected: boolean;
  receiving: boolean;
  ownPositionKnown: boolean;
  alerting: boolean | null;
  alertUnknown: number;
  emergency: string;
  errorToken: string;
  units: TrafficUnits;
  contacts: readonly TrafficContact[];
};

const numberOf = (json: Json, name: string): number | null => {
  const value = optDouble(json, name);
  return Number.isFinite(value) ? value : null;
};

const flagOrNull = (json: Json, name: string): boolean | null => (isNull(json, name) ? null : optBoolean(json, name));

export const trafficReading = (view: Json | null): TrafficReading | null => {
  if (view === null || optText(view, 'class') !== 'AdsbTraffic') return null;
  const units = optJSONObject(view, 'units') ?? {};
  return {
    enabled: optBoolean(view, 'enabled'),
    available: optBoolean(view, 'available'),
    connected: optBoolean(view, 'connected'),
    receiving: optBoolean(view, 'receiving'),
    ownPositionKnown: optBoolean(view, 'ownPositionKnown'),
    alerting: flagOrNull(view, 'alerting'),
    alertUnknown: optInt(view, 'alertUnknown'),
    emergency: optText(view, 'emergency'),
    errorToken: optText(optJSONObject(view, 'error'), 'token'),
    units: TrafficUnits({ distance: optText(units, 'distance'), altitude: optText(units, 'altitude'), heading: optText(units, 'heading') }),
    contacts: jsonObjects(optJSONArray(view, 'contacts')).map((contact) =>
      TrafficContact({
        icaoAddress: optInt(contact, 'icaoAddress'),
        callsign: optText(contact, 'callsign').trim(),
        distance: numberOf(contact, 'distance'),
        bearingDegrees: numberOf(contact, 'bearingDegrees'),
        altitude: numberOf(contact, 'altitude'),
        relativeAltitude: numberOf(contact, 'relativeAltitude'),
        emergency: optText(contact, 'emergency'),
        alert: flagOrNull(contact, 'alert'),
        stale: optBoolean(contact, 'stale'),
        altitudeType: optText(contact, 'altitudeType'),
        simulated: optBoolean(contact, 'simulated'),
      }),
    ),
  };
};

export const trafficShown = (reading: TrafficReading): boolean => reading.enabled || reading.contacts.length > 0;

export const trafficLevel = (reading: TrafficReading): TrafficLevel => {
  if (reading.emergency.trim() !== '') return TrafficLevel.Critical;
  if (reading.alerting === true) return TrafficLevel.Warning;
  if (reading.errorToken.trim() !== '') return TrafficLevel.Caution;
  if (!reading.receiving) return TrafficLevel.Caution;
  if (reading.alertUnknown > 0 && reading.connected) return TrafficLevel.Caution;
  return TrafficLevel.Good;
};

export const trafficReal = (reading: TrafficReading): number => reading.contacts.filter((it) => !it.simulated).length;

export const trafficSynthetic = (reading: TrafficReading): number => reading.contacts.filter((it) => it.simulated).length;

const feedFault = (reading: TrafficReading): string | null => {
  if (reading.errorToken === 'connectFailed') return 'Traffic server unreachable';
  if (reading.errorToken === 'linkLost') return 'Traffic feed dropped';
  if (reading.errorToken.trim() !== '') return 'Traffic feed failed';
  if (!reading.available) return 'No traffic receiver';
  if (!reading.receiving) return 'No traffic feed';
  return null;
};

export const trafficSummary = (reading: TrafficReading): string => {
  const real = trafficReal(reading);
  const synthetic = trafficSynthetic(reading);
  if (real > 0 && synthetic > 0) return `Traffic: ${real} aircraft · ${synthetic} simulated`;
  if (real > 0) return `Traffic: ${real} aircraft`;
  if (synthetic > 0) return `Traffic: ${synthetic} simulated`;
  return feedFault(reading) ?? 'Traffic clear';
};

export const trafficContactUrgent = (contact: TrafficContact): boolean => contact.alert === true || contact.emergency.trim() !== '';

export const trafficCaption = (reading: TrafficReading): string =>
  reading.ownPositionKnown ? 'Range, bearing and height are relative to the vehicle' : 'No vehicle position, so nothing can be ranged';

export const trafficEmergencyText = (token: string): string => {
  switch (token) {
    case 'hijack':
      return 'squawking hijack';
    case 'radioFailure':
      return 'squawking radio failure';
    case 'general':
      return 'squawking emergency';
    default:
      return '';
  }
};

const reading = (value: number | null, unit: string, fine: boolean = false): string => {
  if (value === null) return '';
  const printed = fine && Math.abs(value) < 10.0 ? format('%.1f', value) : format('%.0f', value);
  return unit.trim() === '' ? printed : `${printed} ${unit}`;
};

export const trafficDatumText = (altitudeType: string): string => {
  switch (altitudeType) {
    case 'pressureQnh':
      return 'by pressure';
    case 'geometric':
      return 'by GPS';
    default:
      return '';
  }
};

export const trafficHeightText = (contact: TrafficContact, units: TrafficUnits): string => {
  const relative = contact.relativeAltitude;
  if (relative === null) return [reading(contact.altitude, units.altitude), trafficDatumText(contact.altitudeType)].filter((it) => it.trim() !== '').join(' ');
  const printed = reading(Math.abs(relative), units.altitude);
  if (Math.abs(relative) < 0.5) return 'my level';
  return relative > 0 ? `${printed} above` : `${printed} below`;
};

export const trafficContactText = (contact: TrafficContact, units: TrafficUnits): string => {
  const position = contact.located
    ? [reading(contact.distance, units.distance, true), reading(contact.bearingDegrees, units.heading), trafficHeightText(contact, units)]
    : ['bearing unknown', trafficHeightText(contact, units)];
  const trailing = [trafficEmergencyText(contact.emergency), contact.alert === true ? 'alerting' : '', contact.stale ? 'stale' : ''];
  const synthetic = contact.simulated ? 'simulated' : '';
  return [synthetic, ...position, ...trailing].filter((it) => it.trim() !== '').join('  ');
};

export type TrafficAlert = { level: TrafficLevel; title: string; detail: string };

export const TrafficAlert = (level: TrafficLevel, title: string, detail: string): TrafficAlert => ({ level, title, detail });

const COMPASS_POINTS: readonly string[] = ['N', 'NE', 'E', 'SE', 'S', 'SW', 'W', 'NW'];

const floorMod = (value: number, modulus: number): number => ((value % modulus) + modulus) % modulus;

export const compassPoint = (bearingDegrees: number): string => COMPASS_POINTS[floorMod(Math.round(bearingDegrees / 45.0), COMPASS_POINTS.length)]!;

export const trafficThreat = (reading: TrafficReading): TrafficContact | null =>
  reading.contacts
    .filter(trafficContactUrgent)
    .reduce<TrafficContact | null>((best, it) => (best === null || (it.distance ?? Number.MAX_VALUE) < (best.distance ?? Number.MAX_VALUE) ? it : best), null);

export const trafficThreatText = (contact: TrafficContact, units: TrafficUnits): string => {
  const distance = reading(contact.distance, units.distance, true);
  const where = [...(distance.trim() === '' ? [] : [distance]), ...(contact.bearingDegrees === null ? [] : [compassPoint(contact.bearingDegrees)])].join(' ');
  return [contact.name, where, trafficHeightText(contact, units)].filter((it) => it.trim() !== '').join(' · ');
};

export const trafficAlert = (reading: TrafficReading): TrafficAlert | null => {
  const level = trafficLevel(reading);
  if (!trafficShown(reading) || level === TrafficLevel.Good) return null;
  const threat = level.ordinal >= TrafficLevel.Warning.ordinal ? trafficThreat(reading) : null;
  const title =
    level === TrafficLevel.Critical
      ? `Aircraft ${trafficEmergencyText(reading.emergency).trim() === '' ? 'in emergency' : trafficEmergencyText(reading.emergency)}`
      : level === TrafficLevel.Warning
        ? 'Aircraft nearby'
        : (feedFault(reading) ?? `Collision alerts unknown for ${reading.alertUnknown} aircraft`);
  return TrafficAlert(level, title, threat === null ? '' : trafficThreatText(threat, reading.units));
};

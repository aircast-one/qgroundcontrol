import { type Json, isNull, optBoolean, optText } from '@/bridge/Json';

export const ORBIT_VIEW = 'view.orbit';

export type OrbitReading = {
  readonly available: boolean;
  readonly orbiting: boolean | null;
  readonly radiusText: string;
  readonly clockwise: boolean | null;
  readonly reason: string;
};

export const orbitReading = (view: Json | null): OrbitReading | null =>
  view === null || optText(view, 'class') !== 'Orbit'
    ? null
    : {
        available: optBoolean(view, 'available'),
        orbiting: isNull(view, 'orbiting') ? null : optBoolean(view, 'orbiting'),
        radiusText: optText(view, 'radiusText'),
        clockwise: isNull(view, 'clockwise') ? null : optBoolean(view, 'clockwise'),
        reason: optText(view, 'reason'),
      };

const turnOf = (clockwise: boolean | null): string => (clockwise === true ? 'clockwise' : clockwise === false ? 'anticlockwise' : '');

export const orbitLabel = (reading: OrbitReading | null): string | null =>
  reading === null || reading.orbiting !== true ? null : ['Orbiting', reading.radiusText, turnOf(reading.clockwise)].filter((it) => it.trim() !== '').join(' ');

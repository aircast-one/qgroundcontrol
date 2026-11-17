import { mutableStateOf } from '@/kotlin/State';
import type { LossFailsafe } from '@/ui/VehicleLinks';
import type { TrackPoint } from './VehicleMap';
import type { VehicleChoice } from './VehiclePicker';

export type LostVehicle = {
  readonly name: string;
  readonly at: TrackPoint;
  readonly heading: number;
  readonly home: TrackPoint | null;
  readonly altitude: string | null;
  readonly battery: number | null;
  readonly failsafe: LossFailsafe | null;
};

export const lostVehicle = mutableStateOf<LostVehicle | null>(null);

export const LAST_SEEN = 'Last seen';
export const LINK_LOST = 'Link lost';

const ALTITUDE_ROW = 'alt (rel)';

export const lastSeenArmed = (choice: VehicleChoice, battery: number | null, failsafe: LossFailsafe | null): LostVehicle | null =>
  choice.armed
    ? {
        name: choice.name,
        at: { latitude: choice.latitude, longitude: choice.longitude },
        heading: choice.heading,
        home: choice.home,
        altitude: choice.telemetry.find(([label]) => label.trim().toLowerCase() === ALTITUDE_ROW)?.[1] ?? null,
        battery,
        failsafe,
      }
    : null;

export const keptSighting = (held: LostVehicle | null, active: VehicleChoice | null, battery: number | null, failsafe: LossFailsafe | null): LostVehicle | null =>
  active === null ? held : lastSeenArmed(active, battery ?? held?.battery ?? null, failsafe ?? held?.failsafe ?? null);

export type LinkWatch = { readonly sighting: LostVehicle | null; readonly lost: LostVehicle | null };

export const NO_LINK_WATCH: LinkWatch = { sighting: null, lost: null };

export const linkWatchStep = (held: LinkWatch, connected: boolean, active: VehicleChoice | null, battery: number | null, failsafe: LossFailsafe | null): LinkWatch =>
  connected
    ? { sighting: keptSighting(held.sighting, active, battery, failsafe), lost: null }
    : held.sighting === null
      ? held
      : { sighting: null, lost: held.sighting };

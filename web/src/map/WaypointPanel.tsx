import type { ClosedRange } from './SettingStepper';
import type { ItemRow } from './PlanItems';

const FEET = 'ft';
const ALTITUDE_CEILING_METRES = 500.0;
const ALTITUDE_CEILING_FEET = 1640.0;
const HOME_ITEM = 0;

export const altitudeRange = (units: string): ClosedRange => ({ start: 0.0, endInclusive: units === FEET ? ALTITUDE_CEILING_FEET : ALTITUDE_CEILING_METRES });

export const stripRows = (rows: readonly ItemRow[]): readonly ItemRow[] => rows.filter((it) => it.index !== HOME_ITEM);

import type { Fact } from '@/bridge/Qgc';
import { toDoubleOrNull } from '@/map/AltitudeEntry';

export const factNumber = (fact: Fact): number | null => (typeof fact.value === 'number' ? fact.value : toDoubleOrNull(fact.valueString));

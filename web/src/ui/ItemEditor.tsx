import { type Json, optJSONObject } from '@/bridge/Json';
import type { Fact, FactSlider } from '@/bridge/Qgc';

const HOLD_LABEL = 'Hold';

export type SpeedSection = { specified: boolean; value: number | null; units: string; path: string; specifyPath: string; slider: FactSlider | null };

export const speedSectionOf = (_served: Json | null): SpeedSection | null => null;

export const withoutHeroFields = (fields: readonly Fact[], view: Json | null): readonly Fact[] =>
  optJSONObject(view, 'hold') === null ? fields : fields.filter((it) => it.description !== HOLD_LABEL);

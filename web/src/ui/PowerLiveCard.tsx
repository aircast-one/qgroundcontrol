import { useMemo } from 'react';
import { type Json, jsonObjects, optBoolean, optJSONArray, optText, arrayJSONObject } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { Surface, Text } from '@/m3';
import { md } from '@/theme/Colors';

export type PowerTile = { label: string; value: string; units: string };

export const PowerTile = (label: string, value: string, units: string): PowerTile => ({ label, value, units });

const POWER_TILE_FACTS: readonly (readonly [string, string])[] = [
  ['voltage', 'VOLTAGE'],
  ['current', 'CURRENT'],
  ['mahConsumed', 'USED'],
];

export const powerTiles = (view: Json | null): readonly PowerTile[] => {
  const packs = view !== null && optBoolean(view, 'available') ? optJSONArray(view, 'packs') : null;
  const facts = optJSONArray(arrayJSONObject(packs, 0), 'facts');
  if (facts === null) return [];
  const byName = new Map(jsonObjects(facts).map((it) => [optText(it, 'name'), it] as const));
  return POWER_TILE_FACTS.flatMap(([name, label]) => {
    const fact = byName.get(name);
    if (fact === undefined) return [];
    const units = optText(fact, 'units');
    return [PowerTile(label, optText(fact, 'valueString'), units === 'v' ? 'V' : units)];
  });
};

export const PowerLiveCard = ({ className }: { className?: string }) => {
  const json = useQgcPath('view.battery');
  const tiles = useMemo(() => powerTiles(json), [json]);
  if (tiles.length === 0) return null;
  return (
    <Surface className={`mx-4 my-2 ${className ?? ''}`} color={md('surfaceContainer')} shape="large">
      <div className="flex justify-between p-4">
        {tiles.map((tile) => (
          <div key={tile.label} className="flex flex-col">
            <Text typography="labelSmall" color={md('onSurfaceVariant')}>
              {tile.label}
            </Text>
            <div className="flex items-end gap-1">
              <Text typography="titleLarge">{tile.value}</Text>
              <Text typography="labelMedium" color={md('onSurfaceVariant')} className="pb-[3px]">
                {tile.units}
              </Text>
            </div>
          </div>
        ))}
      </div>
    </Surface>
  );
};

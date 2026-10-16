import { useState } from 'react';
import { optBoolean } from '@/bridge/Json';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { FilterChip, Switch, Text } from '@/m3';
import { AircastSheet } from './AircastSheet';
import { setOk, settingJson } from './BridgeCalls';
import { MapBridge, useMapPath } from './MapBridge';
import { MAP_TYPES_VIEW, type MapTypes, mapTypes } from './PlanMapContent';

const OBSTACLE_OVERLAY = 'settings.flyViewSettings.showObstacleDistanceOverlay';

export const MapLayersSheet = ({ onDismiss }: { onDismiss: () => void }) => {
  const [listed, setListed] = useState<MapTypes | null>(null);
  const obstacleJson = useMapPath(OBSTACLE_OVERLAY);
  const obstacles = optBoolean(obstacleJson, 'value');
  useLaunchedEffect([], async (scope) => {
    const read = mapTypes(await MapBridge.read(MAP_TYPES_VIEW));
    if (scope.isActive()) setListed(read);
  });
  const choose = async (type: string) => {
    const path = listed?.path;
    if (path === undefined) return;
    await setOk(path, settingJson(JSON.stringify(type)));
    setListed((held: MapTypes | null) => (held === null ? null : { ...held, current: type }));
  };
  const toggleObstacles = () => void setOk(OBSTACLE_OVERLAY, settingJson(String(!obstacles)));
  return (
    <AircastSheet onDismissRequest={onDismiss}>
      <div className="flex flex-col gap-3 px-6 pb-8">
        <div className="flex flex-col">
          <Text typography="headlineSmall">Map layers</Text>
          <Text typography="bodyMedium" className="text-on-surface-variant">
            What the map shows while flying
          </Text>
        </div>
        <div className="flex flex-wrap gap-x-2">
          {listed?.types.map((type: string) => (
            <FilterChip key={type} selected={type === listed.current} onClick={() => void choose(type)} label={type} />
          ))}
        </div>
        <div
          role="switch"
          aria-checked={obstacles}
          tabIndex={0}
          className="flex w-full cursor-pointer items-center py-2"
          onClick={toggleObstacles}
          onKeyDown={(event) => {
            if (event.key !== 'Enter' && event.key !== ' ') return;
            event.preventDefault();
            toggleObstacles();
          }}
        >
          <div className="flex flex-1 flex-col">
            <Text typography="bodyLarge">Obstacle distance</Text>
            <Text typography="bodyMedium" className="text-on-surface-variant">
              Proximity sensor readings around the vehicle
            </Text>
          </div>
          <span inert>
            <Switch checked={obstacles} onCheckedChange={null} />
          </span>
        </div>
      </div>
    </AircastSheet>
  );
};

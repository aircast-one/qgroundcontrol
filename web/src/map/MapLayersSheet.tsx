import { useState } from 'react';
import { optBoolean } from '@/bridge/Json';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { FilterChip, Switch, Text } from '@/m3';
import { AircastSheet } from './AircastSheet';
import { setOk, settingJson } from './BridgeCalls';
import { Map3d, useMap3dShown } from './Map3d';
import { MapBridge, useMapPath } from './MapBridge';
import { MAP_TYPES_VIEW, type MapTypes, mapTypes } from './PlanMapContent';

const OBSTACLE_OVERLAY = 'settings.flyViewSettings.showObstacleDistanceOverlay';
const SYNTHETIC_VIEW = 'view.syntheticView';
const MAP_3D_TITLE = '3D view';
const MAP_3D_DETAIL = 'Terrain and buildings around the aircraft, and its height above the ground';

export const MapLayersSheet = ({ onDismiss }: { onDismiss: () => void }) => {
  const [listed, setListed] = useState<MapTypes | null>(null);
  const shown3d = useMap3dShown();
  const offers3d = optBoolean(useMapPath(SYNTHETIC_VIEW), 'native');
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
        {offers3d ? <LayerSwitch title={MAP_3D_TITLE} detail={MAP_3D_DETAIL} checked={shown3d} onToggle={() => Map3d.show(!shown3d)} /> : null}
        <LayerSwitch
          title="Obstacle distance"
          detail="Proximity sensor readings around the vehicle"
          checked={obstacles}
          onToggle={() => void setOk(OBSTACLE_OVERLAY, settingJson(String(!obstacles)))}
        />
      </div>
    </AircastSheet>
  );
};

const LayerSwitch = ({ title, detail, checked, onToggle }: { title: string; detail: string; checked: boolean; onToggle: () => void }) => (
  <div
    role="switch"
    aria-checked={checked}
    tabIndex={0}
    className="flex w-full cursor-pointer items-center gap-4 py-2"
    onClick={onToggle}
    onKeyDown={(event) => {
      if (event.key !== 'Enter' && event.key !== ' ') return;
      event.preventDefault();
      onToggle();
    }}
  >
    <div className="flex flex-1 flex-col">
      <Text typography="bodyLarge">{title}</Text>
      <Text typography="bodyMedium" className="text-on-surface-variant">
        {detail}
      </Text>
    </div>
    <span inert>
      <Switch checked={checked} onCheckedChange={null} />
    </span>
  </div>
);

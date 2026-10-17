import { useContext, useEffect, useMemo, useRef } from 'react';
import { type Json, jsonObjects, optBoolean, optDouble, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { VehicleCommands } from '@/bridge/VehicleCommands';
import { useKeyedState, useLaunchedEffect } from '@/kotlin/Compose';
import { useValue } from '@/kotlin/State';
import { coerceAtLeast, format } from '@/kotlin/Std';
import { Button, HorizontalDivider, ListItem, OutlinedButton, OutlinedTextField, Slider, Switch, Text, type SheetAnchor } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { typedNumber } from '@/map/AltitudeEntry';
import { LocalFlyMapEdits } from '@/map/FlyMapEdits';
import { type LoiterEdit, loiterEditNumber } from '@/map/GotoMarker';
import { MINIMUM_CIRCLE_RADIUS_METRES } from '@/map/MissionLayers';
import { OrbitCircle } from '@/map/OrbitMarker';
import { sentenceCase } from '@/map/SentenceCase';
import { metresBetween } from '@/map/TerrainProfile';
import type { TrackPoint } from '@/map/VehicleMap';
import { aircast, md } from '@/theme/Colors';
import { useRememberActiveVehicleId } from './ActiveVehicle';
import { HoldOrCancel } from './ConfirmTrack';
import { GUIDED_ALTITUDE, type GuidedAltitude, guidedAltitude, guidedAltitudePath, rangeLabel } from './GuidedAltitude';
import { GuidedValuePanel } from './GuidedValuePanel';
import { EditPositionDialog, geoOf } from './ItemEditor';
import { RangeHint } from './VehicleUi';

export const MAP_CLICK_PATH = 'view.mapClick';

export type MapPoint = { readonly latitude: number; readonly longitude: number };

export const ORBIT_ACTION = 'Orbit';

export const RETURN_HOME_INSTEAD = 'Return home instead';

export const MAP_HOLD_HINT = 'Press and hold the map to fly there';
export const MAP_RIGHT_CLICK_HINT = 'Right-click the map to fly there';

export const mapHoldHintText = (desktop: boolean): string => (desktop ? MAP_RIGHT_CLICK_HINT : MAP_HOLD_HINT);

export type MapHoldHint = { readonly name: 'Unseen' | 'Due' | 'Learned'; readonly ordinal: number };

export const MapHoldHint = {
  Unseen: { name: 'Unseen', ordinal: 0 },
  Due: { name: 'Due', ordinal: 1 },
  Learned: { name: 'Learned', ordinal: 2 },
} as const satisfies Record<string, MapHoldHint>;

export const MapHoldHintEntries: readonly MapHoldHint[] = [MapHoldHint.Unseen, MapHoldHint.Due, MapHoldHint.Learned];

export type GotoPreview = { readonly distance: string; readonly height: string; readonly verdict: string; readonly pastReturnPoint: boolean };

export const gotoPreviewPath = (point: MapPoint): string => format('view.gotoPreview(%.7f,%.7f)', point.latitude, point.longitude);

export const gotoPreview = (view: Json | null): GotoPreview | null =>
  view !== null && optBoolean(view, 'available')
    ? { distance: optText(view, 'distanceText'), height: optText(view, 'heightText'), verdict: optText(view, 'verdict'), pastReturnPoint: optBoolean(view, 'pastReturnPoint') }
    : null;

const notBlank = (text: string): boolean => text.trim() !== '';

export const gotoPreviewLines = (preview: GotoPreview | null, fallback: string): readonly string[] => {
  const lines = preview === null ? [] : [[preview.distance, preview.height].filter(notBlank).join(' · '), preview.verdict].filter(notBlank);
  return lines.length > 0 ? lines : [fallback];
};

export const GOTO_ACTION = 'GoTo';

const COMPASS_POINTS: readonly string[] = ['north', 'north-east', 'east', 'south-east', 'south', 'south-west', 'west', 'north-west'];

const DEGREES_TO_RADIANS = Math.PI / 180;

const RADIANS_TO_DEGREES = 180 / Math.PI;

export const goHereText = (from: MapPoint, to: MapPoint, unit: string, metresPerUnit: number): string | null => {
  if (metresPerUnit <= 0.0 || unit.trim() === '') return null;
  const metres = metresBetween(from, to);
  const fromLat = from.latitude * DEGREES_TO_RADIANS;
  const toLat = to.latitude * DEGREES_TO_RADIANS;
  const deltaLon = (to.longitude - from.longitude) * DEGREES_TO_RADIANS;
  const bearing = (Math.atan2(Math.sin(deltaLon) * Math.cos(toLat), Math.cos(fromLat) * Math.sin(toLat) - Math.sin(fromLat) * Math.cos(toLat) * Math.cos(deltaLon)) * RADIANS_TO_DEGREES + 360.0) % 360.0;
  const point = COMPASS_POINTS[Math.round(bearing / 45.0) % COMPASS_POINTS.length] ?? '';
  return `${Math.round(metres / metresPerUnit)} ${unit} ${point}`;
};

export type MapClickAction = {
  readonly id: string;
  readonly path: string;
  readonly label: string;
  readonly title: string;
  readonly message: string;
  readonly confirm: boolean;
};

export const mapClickActions = (view: Json | null): readonly MapClickAction[] =>
  jsonObjects(optJSONArray(view, 'actions')).map((it) => ({
    id: optText(it, 'id'),
    path: optText(it, 'path'),
    label: optText(it, 'label'),
    title: optText(it, 'title'),
    message: optText(it, 'message'),
    confirm: optBoolean(it, 'confirm', true),
  }));

export type OrbitChoice = { readonly radiusMetres: number; readonly clockwise: boolean; readonly aboveHomeMetres: number };

export type OrbitDefaults = { readonly radius: number; readonly unit: string; readonly metresPerUnit: number; readonly clockwise: boolean };

export const orbitDefaults = (view: Json | null): OrbitDefaults => {
  const radius = optDouble(view, 'orbitDefaultRadius');
  const metresPerUnit = optDouble(view, 'orbitMetresPerUnit');
  return {
    radius: Number.isNaN(radius) ? 0.0 : radius,
    unit: optText(view, 'orbitRadiusUnit'),
    metresPerUnit: !Number.isNaN(metresPerUnit) && metresPerUnit > 0 ? metresPerUnit : 1.0,
    clockwise: view === null ? true : optBoolean(view, 'orbitClockwise', true),
  };
};

export const radiusMetres = (entered: string, defaults: OrbitDefaults): number | null => {
  const typed = typedNumber(entered.trim() === '' ? String(defaults.radius) : entered);
  return typed === null ? null : typed * defaults.metresPerUnit;
};

export const orbitOpened = (point: MapPoint, defaults: OrbitDefaults): OrbitCircle =>
  OrbitCircle({
    centre: { latitude: point.latitude, longitude: point.longitude },
    radiusMetres: coerceAtLeast(radiusMetres('', defaults) ?? 0.0, MINIMUM_CIRCLE_RADIUS_METRES),
    clockwise: defaults.clockwise,
  });

export const orbitEdit = (circle: OrbitCircle, defaults: OrbitDefaults): LoiterEdit => ({
  radiusMetres: circle.radiusMetres,
  clockwise: circle.clockwise,
  unit: defaults.unit,
  metresPerUnit: defaults.metresPerUnit,
});

export const orbitArgs = (point: MapPoint, choice: OrbitChoice): readonly unknown[] => [point.latitude, point.longitude, choice.radiusMetres, choice.clockwise, choice.aboveHomeMetres];

export const coordinateLines = (point: MapPoint): readonly string[] => [format('Lat: %.6f', point.latitude), format('Lon: %.6f', point.longitude)];

const send = (action: MapClickAction, point: MapPoint, orbit: OrbitChoice | null): Promise<string | null> =>
  orbit === null ? Qgc.refusalOf(action.path, { latitude: point.latitude, longitude: point.longitude }) : Qgc.refusalOf(action.path, ...orbitArgs(point, orbit));

export const MapClickMenu = ({ point, anchor, onDismiss }: { point: MapPoint; anchor?: SheetAnchor; onDismiss: () => void }) => {
  const view = useQgcPath(MAP_CLICK_PATH);
  const vehicleCoordinate = useQgcPath('vehicle.coordinate');
  const actions = useMemo(() => mapClickActions(view), [view]);
  const empty = view !== null && actions.length === 0;
  useLaunchedEffect([point.latitude, point.longitude, empty], () => {
    if (empty) onDismiss();
  });
  const vehicleId = useRememberActiveVehicleId();
  const [confirming, setConfirming] = useKeyedState<MapClickAction | null>([point.latitude, point.longitude, vehicleId], () => null);
  useLaunchedEffect([actions], () => {
    const gone = confirming !== null && actions.every((it) => it.id !== confirming.id) ? confirming : null;
    if (gone?.id === ORBIT_ACTION) onDismiss();
    else if (gone !== null) setConfirming(null);
  });
  const [refusal, setRefusal] = useKeyedState<string | null>([point.latitude, point.longitude], () => null);

  const defaults = useMemo(() => orbitDefaults(view), [view]);
  const mapEdits = useContext(LocalFlyMapEdits);
  const previewing = confirming?.id === GOTO_ACTION;
  useEffect(() => {
    mapEdits.gotoPreview.value = previewing ? { latitude: point.latitude, longitude: point.longitude } : null;
    return () => {
      mapEdits.gotoPreview.value = null;
    };
  }, [previewing, point.latitude, point.longitude, mapEdits]);
  const previewJson = useQgcPath(previewing ? gotoPreviewPath(point) : MAP_CLICK_PATH);
  const preview = useMemo(() => (previewing ? gotoPreview(previewJson) : null), [previewJson, previewing]);

  const run = (action: MapClickAction) => {
    void send(action, point, null).then((refused) => (refused === null ? onDismiss() : setRefusal(refused)));
  };

  if (view === null || actions.length === 0) return null;

  if (confirming?.id === ORBIT_ACTION) return <OrbitPanel point={point} action={confirming} defaults={defaults} onDone={onDismiss} />;

  const pending = confirming;
  const vehicleGeo = geoOf(vehicleCoordinate);
  const vehicleAt: MapPoint | null = vehicleGeo === null ? null : { latitude: vehicleGeo[0], longitude: vehicleGeo[1] };
  const away = vehicleAt !== null && pending?.id === GOTO_ACTION ? goHereText(vehicleAt, point, defaults.unit, defaults.metresPerUnit) : null;
  const lines = pending === null ? [] : preview !== null ? gotoPreviewLines({ ...preview, distance: away ?? preview.distance }, pending.message) : [away ?? pending.message];
  const warn = preview?.pastReturnPoint === true;

  return (
    <AircastSheet onDismissRequest={onDismiss} anchor={anchor}>
      <div className="flex w-full flex-col pb-6 desktop:pb-1">
        {pending === null ? (
          actions.map((action, index) => (
            <ListItem
              key={`${index}:${action.id}`}
              headlineContent={sentenceCase(action.label)}
              onClick={() => {
                setRefusal(null);
                if (action.confirm) setConfirming(action);
                else run(action);
              }}
            />
          ))
        ) : (
          <div className="flex flex-col gap-3 px-sheet">
            <Text typography="titleLarge">{sentenceCase(pending.title)}</Text>
            {lines.map((line, index) => (
              <Text key={index} typography="bodyMedium" color={warn && index === lines.length - 1 ? aircast('warning') : md('onSurfaceVariant')}>
                {line}
              </Text>
            ))}
            {warn ? (
              <Button className="w-full" onClick={() => void VehicleCommands.returnToLaunch(false).then(() => onDismiss())}>
                {RETURN_HOME_INSTEAD}
              </Button>
            ) : null}
            <HoldOrCancel label={pending.title} onConfirm={() => run(pending)} onCancel={() => setConfirming(null)} />
          </div>
        )}
        {refusal !== null ? (
          <Text typography="bodyMedium" color={md('error')} className="px-sheet py-2">
            {refusal}
          </Text>
        ) : null}
        <div className="py-2">
          <HorizontalDivider />
        </div>
        {coordinateLines(point).map((line) => (
          <Text key={line} typography="bodySmall" color={md('onSurfaceVariant')} className="self-center">
            {line}
          </Text>
        ))}
      </div>
    </AircastSheet>
  );
};

const OrbitPanel = ({ point, action, defaults, onDone }: { point: MapPoint; action: MapClickAction; defaults: OrbitDefaults; onDone: () => void }) => {
  const mapEdits = useContext(LocalFlyMapEdits);
  const opened = useMemo(() => orbitOpened(point, defaults), [point.latitude, point.longitude]);
  useEffect(() => {
    mapEdits.orbit.value = opened;
    return () => {
      mapEdits.orbit.value = null;
    };
  }, [opened, mapEdits]);
  const circle = useValue(mapEdits.orbit) ?? opened;
  const edit = orbitEdit(circle, defaults);
  const [typed, setTyped] = useKeyedState<string | null>([opened], () => null);
  const [height, setHeight] = useKeyedState<GuidedAltitude | null>([opened], () => null);
  const [target, setTarget] = useKeyedState<number | null>([opened], () => null);
  const [settled, setSettled] = useKeyedState<number | null>([opened], () => null);
  const [refusal, setRefusal] = useKeyedState<string | null>([opened], () => null);
  const targetNow = useRef(target);
  targetNow.current = target;

  useLaunchedEffect([settled], async (scope) => {
    const reading = guidedAltitude(await Qgc.get(settled === null ? GUIDED_ALTITUDE : guidedAltitudePath(settled)));
    if (!scope.isActive()) return;
    setHeight(reading);
    if (targetNow.current === null) setTarget(reading?.current ?? null);
  });

  const commit = () => {
    const above = height?.targetMeters ?? height?.currentMeters ?? null;
    if (above === null) {
      setRefusal('Choose a radius and a height for the orbit.');
      return;
    }
    const centre: MapPoint = { latitude: circle.centre.latitude, longitude: circle.centre.longitude };
    void send(action, centre, { radiusMetres: circle.radiusMetres, clockwise: circle.clockwise, aboveHomeMetres: above }).then((refused) => (refused === null ? onDone() : setRefusal(refused)));
  };

  const range = height === null ? null : rangeLabel(height.minimum, height.maximum, height.unit);

  return (
    <div className="pointer-events-none absolute inset-0 z-30 flex items-end justify-center p-3" style={{ paddingBottom: 'calc(env(safe-area-inset-bottom) + 12px)' }}>
      <GuidedValuePanel
        title={sentenceCase(action.title)}
        sentence={action.message}
        commitLabel={action.title}
        commitEnabled={height !== null}
        onCommit={commit}
        onCancel={onDone}
        className="pointer-events-auto max-w-[560px]"
      >
        <OutlinedTextField
          value={loiterRadiusField(typed, edit)}
          onValueChange={(text) => {
            setTyped(text);
            mapEdits.orbit.value = { ...circle, radiusMetres: loiterTyped(text, edit).radiusMetres };
          }}
          label={['Radius', defaults.unit].filter(notBlank).join(' ')}
          singleLine
          keyboardType="decimal"
          className="w-full"
        />
        <div className="flex items-center">
          <Text className="flex-1">Clockwise</Text>
          <Switch
            checked={circle.clockwise}
            onCheckedChange={(it) => {
              mapEdits.orbit.value = { ...circle, clockwise: it };
            }}
            label="Clockwise"
          />
        </div>
        {height !== null ? (
          <>
            <Text typography="bodySmall">{height.label}</Text>
            <Slider
              value={target ?? height.current ?? 0.0}
              onValueChange={(it) => {
                targetNow.current = it;
                setTarget(it);
              }}
              onValueChangeFinished={() => setSettled(targetNow.current)}
              valueRange={[height.minimum ?? 0.0, height.maximum ?? 0.0]}
              label={height.label}
            />
            {range !== null ? <RangeHint text={range} /> : null}
          </>
        ) : null}
        {refusal !== null ? (
          <Text typography="bodyMedium" color={md('error')}>
            {refusal}
          </Text>
        ) : null}
      </GuidedValuePanel>
    </div>
  );
};

export const SET_WAYPOINT_PATH = 'vehicle.setCurrentMissionSequence';

export const waypointTarget = (sequence: number): number => Math.max(sequence, 1);

export const setWaypointMessage = (sequence: number): string => `Adjust current waypoint to ${waypointTarget(sequence)}`;

export const SetWaypointSheet = ({ sequence, onDismiss }: { sequence: number; onDismiss: () => void }) => {
  const [refusal, setRefusal] = useKeyedState<string | null>([sequence], () => null);
  return (
    <AircastSheet onDismissRequest={onDismiss}>
      <div className="flex w-full flex-col gap-3 px-sheet pb-6">
        <Text typography="titleLarge">Set waypoint</Text>
        <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
          {setWaypointMessage(sequence)}
        </Text>
        <HoldOrCancel
          label="Set waypoint"
          onConfirm={() => void Qgc.refusalOf(SET_WAYPOINT_PATH, waypointTarget(sequence)).then((refused) => (refused === null ? onDismiss() : setRefusal(refused)))}
          onCancel={onDismiss}
        />
        {refusal !== null ? (
          <Text typography="bodyMedium" color={md('error')}>
            {refusal}
          </Text>
        ) : null}
      </div>
    </AircastSheet>
  );
};

export type LoiterOffer = {
  readonly latitude: number;
  readonly longitude: number;
  readonly title: string;
  readonly message: string;
  readonly defaultRadius: number;
  readonly clockwise: boolean;
};

export const loiterOffer = (view: Json | null): LoiterOffer | null => {
  const it = optJSONObject(view, 'loiter');
  return it === null
    ? null
    : {
        latitude: optDouble(it, 'latitude'),
        longitude: optDouble(it, 'longitude'),
        title: optText(it, 'title'),
        message: optText(it, 'message'),
        defaultRadius: optDouble(it, 'defaultRadius', 0.0),
        clockwise: optBoolean(it, 'clockwise', true),
      };
};

export const mapClickUnits = (view: Json | null): OrbitDefaults => orbitDefaults(view);

export const signedLoiterRadius = (metres: number, clockwise: boolean): number => (clockwise ? Math.abs(metres) : -Math.abs(metres));

export const loiterEditOpened = (offer: LoiterOffer, units: OrbitDefaults): LoiterEdit => ({
  radiusMetres: Math.abs(offer.defaultRadius) * units.metresPerUnit,
  clockwise: offer.clockwise,
  unit: units.unit,
  metresPerUnit: units.metresPerUnit,
});

export const loiterRadiusField = (typed: string | null, edit: LoiterEdit): string => {
  const typedNumberOf = typed === null ? null : typedNumber(typed);
  return typed !== null && (typed.trim() === '' || (typedNumberOf !== null && typedNumberOf * edit.metresPerUnit === edit.radiusMetres)) ? typed : loiterEditNumber(edit);
};

export const loiterTyped = (text: string, edit: LoiterEdit): LoiterEdit => {
  const typed = typedNumber(text);
  return typed !== null && typed > 0.0 ? { ...edit, radiusMetres: typed * edit.metresPerUnit } : edit;
};

export const LoiterRadiusPanel = ({ offer, units, onRefused, onDone }: { offer: LoiterOffer; units: OrbitDefaults; onRefused: (message: string) => void; onDone: () => void }) => {
  const mapEdits = useContext(LocalFlyMapEdits);
  const opened = useMemo(() => loiterEditOpened(offer, units), [offer.latitude, offer.longitude, offer.title, offer.message, offer.defaultRadius, offer.clockwise]);
  useEffect(() => {
    mapEdits.gotoLoiter.value = opened;
    return () => {
      mapEdits.gotoLoiter.value = null;
    };
  }, [opened, mapEdits]);
  const edit = useValue(mapEdits.gotoLoiter) ?? opened;
  const [typed, setTyped] = useKeyedState<string | null>([opened], () => null);
  const commit = () => {
    const sent = signedLoiterRadius(edit.radiusMetres, edit.clockwise);
    void Qgc.refusalOf('vehicle.guidedModeGotoLocation', { latitude: offer.latitude, longitude: offer.longitude }, sent).then((refused) => {
      if (refused !== null) onRefused(refused);
      onDone();
    });
  };
  return (
    <GuidedValuePanel title={offer.title} sentence={offer.message} commitLabel={offer.title} commitEnabled onCommit={commit} onCancel={onDone}>
      <OutlinedTextField
        value={loiterRadiusField(typed, edit)}
        onValueChange={(text) => {
          setTyped(text);
          mapEdits.gotoLoiter.value = loiterTyped(text, edit);
        }}
        label={['Radius', edit.unit].filter(notBlank).join(' ')}
        singleLine
        keyboardType="decimal"
        className="w-full"
      />
      <div className="flex items-center">
        <Text className="flex-1">Clockwise</Text>
        <Switch
          checked={edit.clockwise}
          onCheckedChange={(it) => {
            mapEdits.gotoLoiter.value = { ...edit, clockwise: it };
          }}
          label="Clockwise"
        />
      </div>
    </GuidedValuePanel>
  );
};

export const STOP_ROI_PATH = 'vehicle.stopGuidedModeROI';
export const SET_ROI_PATH = 'vehicle.guidedModeROI';

export const RoiSheet = ({ at, onDismiss }: { at: TrackPoint; onDismiss: () => void }) => {
  const [editing, setEditing] = useKeyedState([at.latitude, at.longitude], () => false);
  const [refusal, setRefusal] = useKeyedState<string | null>([at.latitude, at.longitude], () => null);
  const run = (path: string, args: readonly unknown[]) => {
    void Qgc.refusalOf(path, ...args).then((refused) => (refused === null ? onDismiss() : setRefusal(refused)));
  };
  return (
    <>
      {editing ? (
        <EditPositionDialog
          at={at}
          onDismiss={() => setEditing(false)}
          title="Edit ROI Position"
          onMove={(latitude: number, longitude: number) => {
            setEditing(false);
            run(SET_ROI_PATH, [{ latitude, longitude }]);
          }}
        />
      ) : null}
      <AircastSheet onDismissRequest={onDismiss}>
        <div className="flex w-full flex-col gap-3 px-sheet pb-6">
          <Text typography="titleMedium">ROI</Text>
          <div className="flex gap-2">
            <OutlinedButton onClick={() => run(STOP_ROI_PATH, [])}>Cancel ROI</OutlinedButton>
            <OutlinedButton onClick={() => setEditing(true)}>Edit position</OutlinedButton>
          </div>
          {refusal !== null ? (
            <Text typography="bodyMedium" color={md('error')}>
              {refusal}
            </Text>
          ) : null}
        </div>
      </AircastSheet>
    </>
  );
};

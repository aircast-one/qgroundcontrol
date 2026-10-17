import { useMemo, useRef, useState } from 'react';
import { Dialog } from '@base-ui/react';
import { type Json, jsonObjects, jsonStrings, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optLong, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useLaunchedEffect, useTicker } from '@/kotlin/Compose';
import { delay, format, roundToInt } from '@/kotlin/Std';
import {
  AlertDialog,
  Button,
  Checkbox,
  CircularProgressIndicator,
  DropdownMenu,
  DropdownMenuItem,
  HorizontalDivider,
  OutlinedButton,
  OutlinedTextField,
  RangeSlider,
  Surface,
  Text,
  TextButton,
} from '@/m3';
import { aircast, md } from '@/theme/Colors';
import { readCamera } from '@/map/FlyMap';
import { useMapPath } from '@/map/MapBridge';
import { OperatorBridge, operatorHeading, operatorPoint } from '@/map/OperatorPosition';
import { CenterMenu } from '@/map/PlanMapContent';
import { currentMapType, qgcRasterStyle } from '@/map/QgcTileSource';
import { type TrackPoint, VehicleMap } from '@/map/VehicleMap';
import { VEHICLES_VIEW, vehicleChoices } from '@/map/VehiclePicker';
import { FootNote } from '@/ui/SetupRow';
import { TileSetTransfer } from '@/ui/TileSetTransfer';

export const OFFLINE_MAPS_GROUP = 'offlineMapsSettings';
export const OFFLINE_MAPS_VIEW = 'view.offlineMaps';
export const OFFLINE_START = 'offlineMaps.startDownload';
export const OFFLINE_RESUME = 'offlineMaps.resume';
export const OFFLINE_CANCEL = 'offlineMaps.cancel';
export const OFFLINE_DELETE = 'offlineMaps.delete';
export const OFFLINE_RENAME = 'offlineMaps.rename';

export const renameWanted = (current: string, typed: string): string | null => {
  const wanted = typed.trim();
  return wanted !== '' && wanted !== current ? wanted : null;
};

const OFFLINE_POLL_MS = 1000;
const MIN_ZOOM_PATH = 'settings.offlineMapsSettings.minZoomLevelDownload';
const MAX_ZOOM_PATH = 'settings.offlineMapsSettings.maxZoomLevelDownload';
const DEFAULT_MIN_ZOOM = 13;
const DEFAULT_MAX_ZOOM = 19;
const SLIDER_MIN_ZOOM = 1;
const SLIDER_MAX_ZOOM = 20;

export type OfflineSet = {
  id: number;
  name: string;
  mapType: string;
  defaultSet: boolean;
  zoomText: string;
  totalText: string;
  uniqueText: string;
  uniqueCount: number;
  downloadedText: string;
  sizeText: string;
  tileCountText: string;
  errorCount: number;
  errorCountText: string;
  downloadStatus: string;
  downloading: boolean;
  complete: boolean;
  subtitle: string;
  rowText: string;
  canDelete: boolean;
};

export type OfflineEstimate = { tileCountText: string; tileSizeText: string; tooMany: boolean };

export type OfflineMaps = {
  available: boolean;
  reason: string;
  sets: readonly OfflineSet[];
  mapList: readonly string[];
  uniqueName: string;
  takenNames: readonly string[];
  estimate: OfflineEstimate | null;
};

export type OfflineRegion = { west: number; north: number; east: number; south: number };

export const offlineMaps = (view: Json | null): OfflineMaps | null => {
  if (view === null || optText(view, 'class') !== 'OfflineMaps') return null;
  const estimate = optJSONObject(view, 'estimate');
  return {
    available: optBoolean(view, 'available'),
    reason: optText(view, 'reason'),
    sets: jsonObjects(optJSONArray(view, 'sets')).map((set) => ({
      id: optLong(set, 'id'),
      name: optText(set, 'name'),
      mapType: optText(set, 'mapTypeStr'),
      defaultSet: optBoolean(set, 'defaultSet'),
      zoomText: optText(set, 'zoomText'),
      totalText: optText(set, 'totalText'),
      uniqueText: optText(set, 'uniqueText'),
      uniqueCount: optLong(set, 'uniqueCount'),
      downloadedText: optText(set, 'downloadedText'),
      sizeText: optText(set, 'sizeText'),
      tileCountText: optText(set, 'tileCountText'),
      errorCount: optInt(set, 'errorCount'),
      errorCountText: optText(set, 'errorCountText'),
      downloadStatus: optText(set, 'downloadStatus'),
      downloading: optBoolean(set, 'downloading'),
      complete: optBoolean(set, 'complete'),
      subtitle: optText(set, 'subtitle'),
      rowText: optText(set, 'rowText'),
      canDelete: optBoolean(set, 'canDelete'),
    })),
    mapList: jsonStrings(optJSONArray(view, 'mapList')),
    uniqueName: optText(view, 'uniqueName'),
    takenNames: jsonStrings(optJSONArray(view, 'takenNames')),
    estimate: estimate === null ? null : { tileCountText: optText(estimate, 'tileCountText'), tileSizeText: optText(estimate, 'tileSizeText'), tooMany: optBoolean(estimate, 'tooMany') },
  };
};

export const offlineRegion = (corners: readonly TrackPoint[]): OfflineRegion | null =>
  corners.length === 0
    ? null
    : {
        west: Math.min(...corners.map((it) => it.longitude)),
        north: Math.max(...corners.map((it) => it.latitude)),
        east: Math.max(...corners.map((it) => it.longitude)),
        south: Math.min(...corners.map((it) => it.latitude)),
      };

const coordinate = (value: number): string => format('%.7f', value);

export const regionCentre = (region: OfflineRegion): TrackPoint => ({ latitude: (region.north + region.south) / 2.0, longitude: (region.west + region.east) / 2.0 });

export const offlineMapsPath = (mapType: string | null, region: OfflineRegion | null, minZoom: number, maxZoom: number, fetchElevation = true): string =>
  mapType === null || region === null
    ? OFFLINE_MAPS_VIEW
    : `${OFFLINE_MAPS_VIEW}(${mapType},${coordinate(region.west)},${coordinate(region.north)},${coordinate(region.east)},${coordinate(region.south)},${minZoom},${maxZoom},${fetchElevation})`;

const zoomSetting = async (path: string, fallback: number): Promise<number> => {
  const value = optDouble(await Qgc.get(path), 'value');
  return Number.isNaN(value) ? fallback : Math.trunc(value);
};

const act = (path: string, ...args: unknown[]): Promise<string | null> => Qgc.refusalOf(path, ...args);

export const OfflineMapsSection = () => {
  const [maps, setMaps] = useState<OfflineMaps | null>(null);
  const [polls, setPolls] = useState(0);
  const [shown, setShown] = useState<OfflineSet | null>(null);
  const [adding, setAdding] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);
  const poll = () => setPolls((it) => it + 1);

  useTicker(OFFLINE_POLL_MS, [polls], async () => setMaps(offlineMaps(await Qgc.get(OFFLINE_MAPS_VIEW))));

  const read = maps;
  if (read === null) return null;
  if (!read.available) return <FootNote text={read.reason.trim() === '' ? 'There is no map tile cache on this device.' : read.reason} />;
  const current = shown === null ? null : (read.sets.find((it) => it.id === shown.id) ?? shown);
  return (
    <>
      <div className="flex w-full flex-col gap-1 px-4 py-2">
        {read.sets.map((set) => (
          <div key={set.id}>
            <button type="button" onClick={() => setShown(set)} className="flex w-full items-center gap-2 py-2 text-left">
              <Text className="min-w-0 flex-1">{set.name}</Text>
              {set.downloading ? <CircularProgressIndicator size={16} strokeWidth={2} label={set.downloadStatus} /> : null}
              <Text color={md('onSurfaceVariant')}>{set.rowText}</Text>
              {set.rowText !== '' ? <span className="size-2 shrink-0 rounded-full" style={{ background: set.complete ? aircast('success') : aircast('alert') }} /> : null}
            </button>
            <HorizontalDivider />
          </div>
        ))}
        <Button onClick={() => setAdding(true)} className="self-start">
          Add new set
        </Button>
        <TileSetTransfer
          sets={read.sets}
          onRefusal={(it) => {
            setRefusal(it);
            poll();
          }}
        />
        {refusal === null ? null : <Text color={aircast('alert')}>{refusal}</Text>}
      </div>

      {current === null ? null : (
        <OfflineSetDialog
          key={current.id}
          set={current}
          onDismiss={() => setShown(null)}
          onRename={(name) =>
            void (async () => {
              setRefusal(await act(OFFLINE_RENAME, current.id, name));
              poll();
            })()
          }
          onAction={(path) =>
            void (async () => {
              setRefusal(await act(path, current.id));
              if (path === OFFLINE_DELETE) setShown(null);
              poll();
            })()
          }
        />
      )}

      {adding ? (
        <OfflineSetEditor
          onDismiss={() => setAdding(false)}
          onDownload={(name, mapType, region, minZoom, maxZoom, elevation) =>
            void (async () => {
              const refused = await act(OFFLINE_START, name, mapType, region.west, region.north, region.east, region.south, minZoom, maxZoom, elevation);
              setRefusal(refused);
              if (refused === null) setAdding(false);
              poll();
            })()
          }
        />
      ) : null}
    </>
  );
};

const InfoLine = ({ label, value }: { label: string; value: string }) => (
  <div className="flex w-full gap-2">
    <Text color={md('onSurfaceVariant')} style={{ flex: 0.45 }}>
      {label}
    </Text>
    <Text style={{ flex: 0.55 }}>{value}</Text>
  </div>
);

const OfflineSetDialog = ({ set, onDismiss, onRename, onAction }: { set: OfflineSet; onDismiss: () => void; onRename: (name: string) => void; onAction: (path: string) => void }) => {
  const [confirming, setConfirming] = useState(false);
  const [typedName, setTypedName] = useState(set.name);
  if (confirming) {
    return (
      <AlertDialog
        onDismissRequest={() => setConfirming(false)}
        title="Confirm delete"
        text={
          <span className="whitespace-pre-line">
            {set.defaultSet
              ? 'This will delete all tiles INCLUDING the tile sets you have created yourself.\n\nIs this really what you want?'
              : `Delete ${set.name} and all its tiles.\n\nIs this really what you want?`}
          </span>
        }
        confirmButton={
          <TextButton
            onClick={() => {
              setConfirming(false);
              onAction(OFFLINE_DELETE);
            }}
          >
            Delete
          </TextButton>
        }
        dismissButton={<TextButton onClick={() => setConfirming(false)}>Cancel</TextButton>}
      />
    );
  }
  return (
    <AlertDialog
      onDismissRequest={onDismiss}
      title={set.name}
      text={
        <div className="flex flex-col gap-1.5">
          {set.defaultSet ? (
            <>
              <Text color={md('onSurfaceVariant')}>{set.subtitle}</Text>
              <InfoLine label="Size:" value={set.sizeText} />
              <InfoLine label="Tile Count:" value={set.tileCountText} />
            </>
          ) : (
            <>
              <OutlinedTextField value={typedName} onValueChange={setTypedName} label="Name" singleLine />
              <Text color={md('onSurfaceVariant')}>{set.mapType}</Text>
              <InfoLine label="Zoom Levels:" value={set.zoomText} />
              <InfoLine label="Total:" value={set.totalText} />
              {set.uniqueCount > 0 ? <InfoLine label="Unique:" value={set.uniqueText} /> : null}
              {!set.complete ? <InfoLine label="Downloaded:" value={set.downloadedText} /> : null}
              {!set.complete && set.errorCount > 0 ? <InfoLine label="Error Count:" value={set.errorCountText} /> : null}
            </>
          )}
        </div>
      }
      confirmButton={
        <div className="flex flex-wrap justify-end gap-1">
          {!set.defaultSet && !set.complete && !set.downloading ? <TextButton onClick={() => onAction(OFFLINE_RESUME)}>Resume download</TextButton> : null}
          {!set.defaultSet && set.downloading ? <TextButton onClick={() => onAction(OFFLINE_CANCEL)}>Cancel download</TextButton> : null}
          <TextButton onClick={() => setConfirming(true)} enabled={set.canDelete}>
            Delete
          </TextButton>
          {!set.defaultSet ? (
            <TextButton
              enabled={typedName.trim() !== ''}
              onClick={() => {
                const wanted = renameWanted(set.name, typedName);
                if (wanted !== null) onRename(wanted);
                onDismiss();
              }}
            >
              Ok
            </TextButton>
          ) : null}
          <TextButton onClick={onDismiss}>{set.defaultSet ? 'Close' : 'Cancel'}</TextButton>
        </div>
      }
    />
  );
};

type OperatorFix = { point: TrackPoint; heading: number };

type Zooms = readonly [number, number];

const OfflineSetEditor = ({ onDismiss, onDownload }: { onDismiss: () => void; onDownload: (name: string, mapType: string, region: OfflineRegion, minZoom: number, maxZoom: number, elevation: boolean) => void }) => {
  const [mapType, setMapType] = useState<string | null>(null);
  const [zooms, setZooms] = useState<Zooms>([DEFAULT_MIN_ZOOM, DEFAULT_MAX_ZOOM]);
  const latestZooms = useRef<Zooms>(zooms);
  const [region, setRegion] = useState<OfflineRegion | null>(null);
  const [name, setName] = useState<string | null>(null);
  const [read, setRead] = useState<OfflineMaps | null>(null);
  const flightMap = useMemo(() => readCamera(), []);
  const [operator, setOperator] = useState<OperatorFix | null>(null);
  const [operatorCentre, setOperatorCentre] = useState<TrackPoint | null>(null);
  const [typeMenu, setTypeMenu] = useState(false);
  const [showPreview, setShowPreview] = useState(false);
  const [fetchElevation, setFetchElevation] = useState(true);
  const [picked, setPicked] = useState<TrackPoint | null>(null);
  const [picks, setPicks] = useState(0);
  const fleetJson = useMapPath(VEHICLES_VIEW);
  const home = useMemo(() => vehicleChoices(fleetJson).active?.home ?? null, [fleetJson]);
  const minZoom = Math.trunc(zooms[0]);
  const maxZoom = Math.trunc(zooms[1]);

  useLaunchedEffect([], async (scope) => {
    const [type, min, max] = await Promise.all([currentMapType(), zoomSetting(MIN_ZOOM_PATH, DEFAULT_MIN_ZOOM), zoomSetting(MAX_ZOOM_PATH, DEFAULT_MAX_ZOOM)]);
    if (!scope.isActive()) return;
    setMapType((held) => held ?? type);
    latestZooms.current = [min, max];
    setZooms([min, max]);
  });

  useLaunchedEffect([region !== null], async (scope) => {
    if (region === null) return;
    const follow = async (): Promise<void> => {
      if (!scope.isActive()) return;
      const view = await OperatorBridge.read();
      const at = operatorPoint(view);
      const fix = at === null ? null : { point: at, heading: operatorHeading(view) };
      if (!scope.isActive()) return;
      setOperator(fix);
      setOperatorCentre((held) => held ?? fix?.point ?? null);
      await delay(OFFLINE_POLL_MS);
      return follow();
    };
    await follow();
  });

  useLaunchedEffect([mapType, region?.west, region?.north, region?.east, region?.south, minZoom, maxZoom, fetchElevation], async (scope) => {
    const next = offlineMaps(await Qgc.get(offlineMapsPath(mapType, region, minZoom, maxZoom, fetchElevation)));
    if (!scope.isActive()) return;
    setRead(next);
    setName((held) => held ?? next?.uniqueName ?? null);
  });

  const chosenName = name ?? '';
  const nameTaken = read?.takenNames.includes(chosenName.trim()) === true;
  const estimate = read?.estimate ?? null;
  const previewAt = region === null ? null : regionCentre(region);
  const mapStyle = mapType === null ? null : qgcRasterStyle(mapType);

  return (
    <Dialog.Root open onOpenChange={(open, details) => !open && details.reason !== 'outside-press' && onDismiss()}>
      <Dialog.Portal>
        <Dialog.Popup aria-label="Add new set" className="fixed inset-0 z-50 outline-none">
          <Surface color={md('surface')} contentColor={md('onSurface')} className="flex size-full flex-col">
            <div className="relative min-h-0 w-full flex-1">
              {mapStyle === null ? null : (
                <VehicleMap
                  className="size-full"
                  mapStyle={mapStyle}
                  follow={false}
                  centreRequest={picks > 0 ? 2 + picks : operatorCentre !== null ? 2 : flightMap !== null ? 1 : 0}
                  centreOn={picked ?? operatorCentre ?? flightMap?.centre ?? null}
                  centreZoom={picked === null && operatorCentre === null ? (flightMap?.zoom ?? null) : null}
                  operator={operator?.point ?? null}
                  operatorHeading={operator?.heading ?? NaN}
                  onViewChanged={(corners) => setRegion(offlineRegion(corners))}
                />
              )}
              <div className="absolute top-0 left-0 p-2">
                <CenterMenu
                  launch={home}
                  myLocation={operator?.point ?? null}
                  launchLabel="Home"
                  onCentre={(point) => {
                    setPicked(point);
                    setPicks((it) => it + 1);
                  }}
                />
              </div>
            </div>
            <div className="flex w-full shrink flex-col gap-2 overflow-y-auto p-4">
              <Text typography="titleMedium">Add new set</Text>
              <OutlinedTextField value={chosenName} onValueChange={setName} label="Name:" singleLine isError={nameTaken} className="w-full" />
              <div className="relative self-start">
                <OutlinedButton onClick={() => setTypeMenu(true)}>{`Map type: ${mapType ?? ''}`}</OutlinedButton>
                <DropdownMenu expanded={typeMenu} onDismissRequest={() => setTypeMenu(false)}>
                  {(read?.mapList ?? []).map((option) => (
                    <DropdownMenuItem
                      key={option}
                      text={option}
                      onClick={() => {
                        setTypeMenu(false);
                        setMapType(option);
                      }}
                    />
                  ))}
                </DropdownMenu>
              </div>
              <Text>Min/Max Zoom Levels</Text>
              <RangeSlider
                value={[zooms[0], zooms[1]]}
                onValueChange={(it) => {
                  latestZooms.current = it;
                  setZooms(it);
                }}
                onValueChangeFinished={() => {
                  const [min, max] = latestZooms.current;
                  void Qgc.set(MIN_ZOOM_PATH, roundToInt(min)).then(() => Qgc.set(MAX_ZOOM_PATH, roundToInt(max)));
                }}
                valueRange={[SLIDER_MIN_ZOOM, SLIDER_MAX_ZOOM]}
                steps={SLIDER_MAX_ZOOM - SLIDER_MIN_ZOOM - 1}
                label="Min/Max Zoom Levels"
              />
              <div className="flex gap-4">
                <Text>{`Min zoom: ${minZoom}`}</Text>
                <Text>{`Max zoom: ${maxZoom}`}</Text>
              </div>
              {previewAt === null || !showPreview || mapStyle === null ? (
                <OutlinedButton onClick={() => setShowPreview(true)} enabled={previewAt !== null} className="self-start">
                  Show zoom previews
                </OutlinedButton>
              ) : (
                <div className="flex gap-2">
                  {(
                    [
                      [`Min Zoom: ${minZoom}`, minZoom],
                      [`Max Zoom: ${maxZoom}`, maxZoom],
                    ] as const
                  ).map(([label, zoom]) => (
                    <button key={label} type="button" aria-label={label} onClick={() => setShowPreview(false)} className="relative size-[150px] shrink-0">
                      <VehicleMap className="size-full" mapStyle={mapStyle} follow={false} centreRequest={zoom + 1} centreOn={previewAt} centreZoom={zoom} gestures={false} />
                      <Text typography="labelMedium" className="absolute bottom-0 left-1/2 -translate-x-1/2 p-1">
                        {label}
                      </Text>
                    </button>
                  ))}
                </div>
              )}
              {estimate === null ? null : (
                <>
                  <InfoLine label="Tile Count:" value={estimate.tileCountText} />
                  <InfoLine label="Est Size:" value={estimate.tileSizeText} />
                  {estimate.tooMany ? <Text color={aircast('alert')}>Too many tiles</Text> : null}
                </>
              )}
              <div className="flex items-center gap-3">
                <Checkbox checked={fetchElevation} onCheckedChange={setFetchElevation} label="Fetch elevation data" />
                <Text>Fetch elevation data</Text>
              </div>
              {nameTaken ? <Text color={aircast('alert')}>Tile set with this name already exists</Text> : null}
              <div className="flex gap-2">
                <Button
                  onClick={() => {
                    if (region !== null && mapType !== null) onDownload(chosenName.trim(), mapType, region, minZoom, maxZoom, fetchElevation);
                  }}
                  enabled={region !== null && estimate !== null && !estimate.tooMany && !nameTaken && chosenName.trim() !== ''}
                >
                  Download
                </Button>
                <OutlinedButton onClick={onDismiss}>Cancel</OutlinedButton>
              </div>
            </div>
          </Surface>
        </Dialog.Popup>
      </Dialog.Portal>
    </Dialog.Root>
  );
};

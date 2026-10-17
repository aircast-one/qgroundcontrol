import { useEffect, useRef } from 'react';
import { FilterChip, Text } from '@/m3';
import { aircast, argb, md } from '@/theme/Colors';
import { AltitudeModePicker, itemReferenceSelectable, itemReferenceShown } from './AltitudeModePicker';
import { setOk, settingJson } from './BridgeCalls';
import { useMapPath } from './MapBridge';
import { HOME_ITEM } from './MissionKinds';
import { type MissionItem, PlanBridge } from './PlanBridge';
import type { ItemRow } from './PlanItems';
import { sentenceCase } from './SentenceCase';
import { type ClosedRange, SettingStepper } from './SettingStepper';
import { waypointYaw, WaypointActions } from './WaypointActions';
import { waypointHold, waypointSpeed } from './WaypointSpeed';

const CHIP_SIZE = 40;
const STRIP_PADDING = 4;
const FEET = 'ft';
const ALTITUDE_CEILING_METRES = 500.0;
const ALTITUDE_CEILING_FEET = 1640.0;
const SPEED_STEP = 0.5;
export const SPEED_RANGE: ClosedRange = { start: 0.0, endInclusive: 30.0 };

export const altitudeRange = (units: string): ClosedRange => ({ start: 0.0, endInclusive: units === FEET ? ALTITUDE_CEILING_FEET : ALTITUDE_CEILING_METRES });

export const stripRows = (rows: readonly ItemRow[]): readonly ItemRow[] => rows.filter((it) => it.index !== HOME_ITEM);

const parsedColour = (text: string): string | null =>
  /^#[0-9a-fA-F]{6}$/.test(text) ? text : /^#[0-9a-fA-F]{8}$/.test(text) ? argb(Number.parseInt(text.slice(1), 16)) : null;

const reducedMotion = (): boolean => window.matchMedia('(prefers-reduced-motion: reduce)').matches;

export const WaypointStrip = ({
  rows,
  altitudes,
  conflicts,
  selected,
  onPick,
  className,
}: {
  rows: readonly ItemRow[];
  altitudes: ReadonlyMap<number, string>;
  conflicts: ReadonlySet<number>;
  selected: number | null;
  onPick: (index: number) => void;
  className?: string;
}) => {
  const shown = stripRows(rows);
  const list = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const at = shown.findIndex((it) => it.index === selected);
    const child = at >= 0 ? list.current?.children[at] : undefined;
    if (child instanceof HTMLElement) list.current?.scrollTo({ left: child.offsetLeft - STRIP_PADDING, behavior: reducedMotion() ? 'auto' : 'smooth' });
  }, [selected]);
  return (
    <div ref={list} className={`relative flex gap-2 overflow-x-auto ${className ?? ''}`} style={{ paddingInline: STRIP_PADDING }}>
      {shown.map((row) => {
        const chosen = row.index === selected;
        const ink = parsedColour(row.colour) ?? md('primary');
        const conflicting = conflicts.has(row.index);
        const border = conflicting
          ? `3px solid ${md('error')}`
          : chosen
            ? `3px solid ${md('onSurface')}`
            : row.readyForSave
              ? undefined
              : `1px solid ${aircast('warning')}`;
        return (
          <div key={row.index} className="flex shrink-0 flex-col items-center">
            <button
              type="button"
              onClick={() => onPick(row.index)}
              aria-label={`${sentenceCase(row.name)} ${row.seal}${conflicting ? ', too close to the terrain' : ''}`}
              aria-pressed={chosen}
              className="grid place-items-center rounded-full"
              style={{
                width: CHIP_SIZE,
                height: CHIP_SIZE,
                background: row.readyForSave ? ink : 'transparent',
                color: row.readyForSave ? md('surface') : aircast('warning'),
                border,
              }}
            >
              <Text typography="labelLarge" maxLines={1}>
                {row.seal}
              </Text>
            </button>
            <Text typography="labelSmall" color={md('onSurfaceVariant')} maxLines={1}>
              {altitudes.get(row.index) ?? ''}
            </Text>
          </div>
        );
      })}
    </div>
  );
};

export const WaypointSettings = ({
  item,
  globalFrame,
  onWrite,
}: {
  item: MissionItem;
  globalFrame: number | null;
  onWrite: (label: string | null, work: () => Promise<boolean>) => void;
}) => {
  const json = useMapPath(`view.itemFacts(${item.index})`);
  const units = item.altitudeEditUnits.trim() === '' ? 'm' : item.altitudeEditUnits;
  const speed = waypointSpeed(json);
  return (
    <div className="flex w-full flex-col px-3">
      {!Number.isNaN(item.altitude) && item.index !== HOME_ITEM ? (
        <SettingStepper
          label="Altitude"
          value={item.altitude}
          unit={units}
          step={1.0}
          note={item.altitudeFrameText.trim() === '' ? null : item.altitudeFrameText}
          range={altitudeRange(units)}
          slider
          onSet={(metres) => onWrite(null, () => PlanBridge.setAltitude(item.index, metres))}
        />
      ) : null}
      {speed !== null ? (
        <SettingStepper
          label="Speed"
          value={speed.value}
          unit={speed.units}
          step={SPEED_STEP}
          note={speed.specified ? null : 'Mission speed'}
          range={SPEED_RANGE}
          onSet={(wanted) => onWrite(null, async () => (await setOk(speed.specifyPath, settingJson('true'))) && (await setOk(speed.path, settingJson(`${wanted}`))))}
          trailing={speed.specified ? <FilterChip selected={false} onClick={() => onWrite('Using the mission speed', () => setOk(speed.specifyPath, settingJson('false')))} label="Auto" /> : null}
        />
      ) : null}
      {!Number.isNaN(item.altitude) && itemReferenceShown(globalFrame) && item.index !== HOME_ITEM ? (
        <div className="flex w-full items-center py-1">
          <Text typography="bodyLarge" className="flex-1">
            Altitude mode
          </Text>
          <AltitudeModePicker
            item={item}
            onPick={(raw) => onWrite('Setting the altitude frame', () => PlanBridge.setAltitudeMode(item.index, raw))}
            globalFrameMixed={itemReferenceSelectable(globalFrame)}
          />
        </div>
      ) : null}
      {item.index !== HOME_ITEM ? <WaypointActions index={item.index} hold={waypointHold(json)} yaw={waypointYaw(json)} /> : null}
    </div>
  );
};

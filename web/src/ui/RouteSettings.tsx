import { useMemo, useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { type Json, optJSONObject, optText } from '@/bridge/Json';
import { FilterChip, Text } from '@/m3';
import { altitudeCeiling } from '@/map/AltitudeEntry';
import { type ClosedRange, SettingStepper } from '@/map/SettingStepper';
import { MaterialIcon } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { type SpeedSection, speedSectionOf } from '@/ui/ItemEditor';
import { MissionAltitudeFrame } from '@/ui/MissionAltitudeFrame';
import { PlanVehicleRows } from '@/ui/PlanVehicleRows';
import { factFromControl } from '@/ui/ParameterForm';

const ALTITUDE_STEP = 1.0;
const SPEED_STEP = 0.5;
const ROUTE_ALTITUDE_NOTE = 'New waypoints use this';
const VEHICLE_SPEED_NOTE = 'Vehicle default';

export type RouteAltitude = { value: number; units: string; path: string };

const numberOf = (text: string): number | null => (text.trim() === '' || Number.isNaN(Number(text)) ? null : Number(text));

export const routeAltitude = (plan: Json | null): RouteAltitude | null => {
  const control = optJSONObject(optJSONObject(plan, 'defaults'), 'altitude');
  const fact = control === null ? null : factFromControl(control);
  if (fact === null) return null;
  const value = typeof fact.value === 'number' ? fact.value : numberOf(fact.valueString);
  return value === null ? null : { value, units: fact.units, path: fact.path };
};

export const routeSpeedRange = (speed: SpeedSection): ClosedRange => (speed.slider ? { start: speed.slider.from, endInclusive: speed.slider.to } : { start: 0.0, endInclusive: 30.0 });

const ROOMY_HEIGHT = 800;

export const plannedFor = (plan: Json | null): string | null => {
  const vehicle = optJSONObject(plan, 'planningFor');
  const named = [optText(vehicle, 'firmware'), optText(vehicle, 'type')].filter((it) => it.trim() !== '');
  return named.length === 0 ? null : `For ${named.join(' · ')}`;
};

export const RouteSettings = ({ plan }: { plan: Json | null }) => {
  const [refusal, setRefusal] = useState<string | null>(null);
  const [open, setOpen] = useState(() => window.innerHeight >= ROOMY_HEIGHT);
  const summary = plannedFor(plan);
  const write = (work: () => Promise<string | null>) =>
    void (async () => {
      setRefusal(await work());
    })();
  const altitude = useMemo(() => routeAltitude(plan), [plan]);
  const speed = useMemo(() => speedSectionOf(optJSONObject(optJSONObject(plan, 'defaults'), 'flightSpeed')), [plan]);
  return (
    <div className="flex w-full flex-col">
      <button type="button" aria-expanded={open} onClick={() => setOpen(!open)} className="flex min-h-12 w-full items-center gap-2 pl-3 pr-2 text-left">
        <span className="flex min-w-0 flex-1 flex-col">
          <Text typography="titleSmall">Route settings</Text>
          {summary === null ? null : (
            <Text typography="bodySmall" color={md('onSurfaceVariant')} maxLines={1} ellipsis>
              {summary}
            </Text>
          )}
        </span>
        <MaterialIcon name={open ? 'keyboard_arrow_up' : 'keyboard_arrow_down'} filled style={{ color: md('onSurfaceVariant') }} />
      </button>
      {open ? <RouteSettingsBody write={write} altitude={altitude} speed={speed} /> : null}
      {refusal !== null ? (
        <Text typography="bodySmall" color={md('error')} className="block px-3">
          {refusal}
        </Text>
      ) : null}
    </div>
  );
};

const RouteSettingsBody = ({
  write,
  altitude,
  speed,
}: {
  write: (work: () => Promise<string | null>) => void;
  altitude: RouteAltitude | null;
  speed: SpeedSection | null;
}) => (
  <>
    <div className="flex w-full flex-col px-3">
      {altitude !== null ? (
        <SettingStepper
          label="Altitude"
          value={altitude.value}
          unit={altitude.units}
          step={ALTITUDE_STEP}
          note={ROUTE_ALTITUDE_NOTE}
          range={{ start: 0.0, endInclusive: altitudeCeiling(altitude.units) }}
          slider
          onSet={(wanted) => write(() => Qgc.writeRefusal(altitude.path, wanted))}
        />
      ) : null}
      {speed !== null ? (
        <SettingStepper
          label="Speed"
          value={speed.value}
          unit={speed.units}
          step={SPEED_STEP}
          note={!speed.specified ? VEHICLE_SPEED_NOTE : null}
          range={routeSpeedRange(speed)}
          onSet={(wanted) => write(async () => (await Qgc.writeRefusal(speed.specifyPath, true)) ?? Qgc.writeRefusal(speed.path, wanted))}
          trailing={speed.specified ? <FilterChip selected={false} onClick={() => write(() => Qgc.writeRefusal(speed.specifyPath, false))} label="Auto" /> : null}
        />
      ) : null}
    </div>
    <MissionAltitudeFrame />
    <PlanVehicleRows heading="Planned for" />
  </>
);

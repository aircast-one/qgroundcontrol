import { useMemo, useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { type Json, optJSONObject } from '@/bridge/Json';
import { FilterChip, Text } from '@/m3';
import { type ClosedRange, SettingStepper } from '@/map/SettingStepper';
import { md } from '@/theme/Colors';
import { type SpeedSection, speedSectionOf } from '@/ui/ItemEditor';
import { MissionAltitudeFrame } from '@/ui/MissionAltitudeFrame';
import { factFromControl } from '@/ui/ParameterForm';

const ALTITUDE_STEP = 1.0;
const SPEED_STEP = 0.5;
const FEET = 'ft';
const METRES_RANGE: ClosedRange = { start: 0.0, endInclusive: 500.0 };
const FEET_RANGE: ClosedRange = { start: 0.0, endInclusive: 1640.0 };
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

export const RouteSettings = ({ plan }: { plan: Json | null }) => {
  const [refusal, setRefusal] = useState<string | null>(null);
  const write = (work: () => Promise<string | null>) =>
    void (async () => {
      setRefusal(await work());
    })();
  const altitude = useMemo(() => routeAltitude(plan), [plan]);
  const speed = useMemo(() => speedSectionOf(optJSONObject(optJSONObject(plan, 'defaults'), 'flightSpeed')), [plan]);
  return (
    <div className="flex w-full flex-col">
      <Text typography="titleSmall" className="block pt-1 pl-3">
        Route
      </Text>
      <div className="flex w-full flex-col px-3">
        {altitude !== null ? (
          <SettingStepper
            label="Altitude"
            value={altitude.value}
            unit={altitude.units}
            step={ALTITUDE_STEP}
            note={ROUTE_ALTITUDE_NOTE}
            range={altitude.units === FEET ? FEET_RANGE : METRES_RANGE}
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
      {refusal !== null ? (
        <Text typography="bodySmall" color={md('error')} className="block px-3">
          {refusal}
        </Text>
      ) : null}
    </div>
  );
};

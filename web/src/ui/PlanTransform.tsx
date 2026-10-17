import { useState } from 'react';
import { type Json, optDouble, optJSONObject, optString } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { AlertDialog, Checkbox, HorizontalDivider, OutlinedButton, OutlinedTextField, Text, TextButton } from '@/m3';
import { typedNumber } from '@/map/AltitudeEntry';
import { REPOSITION_MISSION } from '@/map/PlanStrip';
import { TrackPoint } from '@/map/VehicleMap';
import { md } from '@/theme/Colors';
import { EditPositionDialog } from '@/ui/ItemEditor';

export const PLAN_TRANSFORM_VIEW = 'view.planTransform';
export const OFFSET_MISSION = 'plan.missionController.offsetMission';
export const ROTATE_MISSION = 'plan.missionController.rotateMission';

export const transformHome = (view: Json | null): TrackPoint | null => {
  const home = optJSONObject(view, 'home');
  return home === null ? null : TrackPoint(optDouble(home, 'latitude'), optDouble(home, 'longitude'));
};

export type DistanceUnit = { name: string; metresPerUnit: number };

const METRES: DistanceUnit = { name: 'm', metresPerUnit: 1.0 };

export const transformUnit = (view: Json | null, axis: string): DistanceUnit => {
  if (view === null) return METRES;
  const name = optString(view, `${axis}Unit`);
  const unit = { name: name.trim() === '' ? 'm' : name, metresPerUnit: optDouble(view, `${axis}MetresPerUnit`) };
  return Number.isFinite(unit.metresPerUnit) && unit.metresPerUnit > 0 ? unit : METRES;
};

export const offsetArgs = (east: string, north: string, up: string, horizontal: DistanceUnit, vertical: DistanceUnit, takeoff: boolean, landing: boolean): readonly (number | boolean)[] | null => {
  const values = (
    [
      [east, horizontal],
      [north, horizontal],
      [up, vertical],
    ] as const
  ).map(([typed, unit]) => {
    const number = typedNumber(typed.trim() === '' ? '0' : typed);
    return number === null ? null : number * unit.metresPerUnit;
  });
  return values.every((it) => it !== null && Number.isFinite(it)) ? [...values.filter((it) => it !== null), takeoff, landing] : null;
};

const NumberField = ({ label, value, onChange }: { label: string; value: string; onChange: (value: string) => void }) => (
  <OutlinedTextField value={value} onValueChange={onChange} label={label} singleLine isError={value.trim() !== '' && typedNumber(value) === null} keyboardType="text" className="w-full" />
);

const CheckRow = ({ label, checked, onChange }: { label: string; checked: boolean; onChange: (checked: boolean) => void }) => (
  <div className="flex items-center gap-3">
    <Checkbox checked={checked} label={label} onCheckedChange={onChange} />
    <Text>{label}</Text>
  </div>
);

export const PlanTransformDialog = ({ onDismiss }: { onDismiss: () => void }) => {
  const view = useQgcPath(PLAN_TRANSFORM_VIEW);
  const home = transformHome(view);
  const horizontal = transformUnit(view, 'horizontal');
  const vertical = transformUnit(view, 'vertical');
  const [east, setEast] = useState('0');
  const [north, setNorth] = useState('0');
  const [up, setUp] = useState('0');
  const [offsetTakeoff, setOffsetTakeoff] = useState(false);
  const [offsetLanding, setOffsetLanding] = useState(false);
  const [degrees, setDegrees] = useState('0');
  const [rotateTakeoff, setRotateTakeoff] = useState(false);
  const [rotateLanding, setRotateLanding] = useState(false);
  const [repositioning, setRepositioning] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);

  const apply = (path: string, args: readonly unknown[]) =>
    void (async () => {
      setRefusal(await Qgc.refusalOf(path, ...args));
    })();

  if (repositioning && home !== null) {
    return (
      <EditPositionDialog
        at={home}
        onDismiss={() => setRepositioning(false)}
        title="Reposition mission"
        confirm="Move to Position"
        vehicleConfirm="Move to Vehicle Position"
        onMove={(latitude, longitude) => {
          setRepositioning(false);
          apply(REPOSITION_MISSION, [{ latitude, longitude }]);
        }}
      />
    );
  }

  const offset = offsetArgs(east, north, up, horizontal, vertical, offsetTakeoff, offsetLanding);
  const rotation = typedNumber(degrees);
  return (
    <AlertDialog
      onDismissRequest={onDismiss}
      title="Transform"
      text={
        <div className="flex flex-col gap-1.5 overflow-y-auto">
          <Text typography="titleSmall">Offset mission</Text>
          <NumberField label={`East (${horizontal.name})`} value={east} onChange={setEast} />
          <NumberField label={`North (${horizontal.name})`} value={north} onChange={setNorth} />
          <NumberField label={`Up (${vertical.name})`} value={up} onChange={setUp} />
          <CheckRow label="Also move takeoff items" checked={offsetTakeoff} onChange={setOffsetTakeoff} />
          <CheckRow label="Also move landing items" checked={offsetLanding} onChange={setOffsetLanding} />
          <Text typography="bodySmall">Note: Home altitude is not modified.</Text>
          <OutlinedButton className="self-start" onClick={() => offset !== null && apply(OFFSET_MISSION, offset)} enabled={offset !== null}>
            Apply offset
          </OutlinedButton>

          <HorizontalDivider />
          <Text typography="titleSmall">Reposition mission</Text>
          {home === null ? <Text typography="bodySmall">Home position must be set to reposition the mission.</Text> : null}
          <OutlinedButton className="self-start" onClick={() => setRepositioning(true)} enabled={home !== null}>
            Move to Position
          </OutlinedButton>

          <HorizontalDivider />
          <Text typography="titleSmall">Rotate mission</Text>
          {home === null ? <Text typography="bodySmall">Home position must be set to rotate the mission.</Text> : null}
          <NumberField label="Clockwise (deg)" value={degrees} onChange={setDegrees} />
          <CheckRow label="Also move takeoff items" checked={rotateTakeoff} onChange={setRotateTakeoff} />
          <CheckRow label="Also move landing items" checked={rotateLanding} onChange={setRotateLanding} />
          <Text typography="bodySmall">Note: Complex items are rotated by moving their reference coordinate: their geometry and orientation are not changed.</Text>
          <OutlinedButton className="self-start" onClick={() => rotation !== null && apply(ROTATE_MISSION, [rotation, rotateTakeoff, rotateLanding])} enabled={home !== null && rotation !== null}>
            Apply rotation
          </OutlinedButton>

          {refusal !== null ? <Text color={md('error')}>{refusal}</Text> : null}
        </div>
      }
      confirmButton={<TextButton onClick={onDismiss}>Close</TextButton>}
    />
  );
};

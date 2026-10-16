import { type Json, optBoolean, optDouble, optJSONObject, optText } from '@/bridge/Json';
import { format } from '@/kotlin/Std';
import { Checkbox, OutlinedTextField, Text } from '@/m3';
import { toDoubleOrNull } from '@/map/AltitudeEntry';
import { FlightMapPosition, type TrackPoint } from '@/map/VehicleMap';

export type FastCompass = {
  invocation: string;
  help: string;
  vehicleHasPosition: boolean;
  gcsLatitude: number | null;
  gcsLongitude: number | null;
};

export type FastCompassChoice = {
  enabled: boolean;
  useGcs: boolean;
  latitude: string;
  longitude: string;
  useMap: boolean;
  mapPosition: TrackPoint | null;
};

export const FastCompassChoice = (fields: { enabled: boolean; useGcs: boolean; latitude: string; longitude: string; useMap?: boolean; mapPosition?: TrackPoint | null }): FastCompassChoice => ({
  useMap: false,
  mapPosition: null,
  ...fields,
});

export const fastCompass = (json: Json | null): FastCompass | null => {
  if (json === null) return null;
  const gcs = optJSONObject(json, 'gcsPosition');
  const valid = optBoolean(gcs, 'valid');
  return {
    invocation: optText(json, 'invocation'),
    help: optText(json, 'help'),
    vehicleHasPosition: optBoolean(json, 'vehicleHasPosition'),
    gcsLatitude: valid ? optDouble(gcs, 'latitude') : null,
    gcsLongitude: valid ? optDouble(gcs, 'longitude') : null,
  };
};

export const initialFastCompassChoice = (fast: FastCompass, mapPosition: TrackPoint | null = FlightMapPosition.latest): FastCompassChoice =>
  FastCompassChoice({ enabled: false, useGcs: fast.gcsLatitude !== null, latitude: '0.00', longitude: '0.00', mapPosition });

export const fastCompassArguments = (fast: FastCompass, choice: FastCompassChoice): readonly unknown[] => {
  if (choice.useGcs && fast.gcsLatitude !== null && fast.gcsLongitude !== null) return [fast.gcsLatitude, fast.gcsLongitude];
  if (choice.useMap && choice.mapPosition !== null) return [choice.mapPosition.latitude, choice.mapPosition.longitude];
  return [toDoubleOrNull(choice.latitude) ?? choice.latitude, toDoubleOrNull(choice.longitude) ?? choice.longitude];
};

export const FastCompassBlock = ({ fast, choice, onChange }: { fast: FastCompass; choice: FastCompassChoice; onChange: (choice: FastCompassChoice) => void }) => {
  const asksForPosition = choice.enabled && !fast.vehicleHasPosition;
  const shownMap = choice.useMap ? choice.mapPosition : null;
  return (
    <div className="flex flex-col gap-2">
      <Text typography="bodySmall">{fast.help}</Text>
      <LabeledCheckbox label="Fast Calibration" checked={choice.enabled} onCheck={(it) => onChange({ ...choice, enabled: it })} />
      {asksForPosition ? <Text typography="bodySmall">Vehicle has no valid position, please provide it</Text> : null}
      {asksForPosition && fast.gcsLatitude !== null ? <LabeledCheckbox label="Use GCS position instead" checked={choice.useGcs} onCheck={(it) => onChange({ ...choice, useGcs: it })} /> : null}
      {asksForPosition && fast.gcsLatitude === null && choice.mapPosition !== null ? (
        <LabeledCheckbox label="Use current map position instead" checked={choice.useMap} onCheck={(it) => onChange({ ...choice, useMap: it })} />
      ) : null}
      {shownMap !== null ? <Text typography="bodySmall">{format('Lat: %.4f Lon: %.4f', shownMap.latitude, shownMap.longitude)}</Text> : null}
      {choice.enabled && !choice.useGcs && shownMap === null ? (
        <>
          <CoordinateField label="Latitude" value={choice.latitude} onValue={(it) => onChange({ ...choice, latitude: it })} />
          <CoordinateField label="Longitude" value={choice.longitude} onValue={(it) => onChange({ ...choice, longitude: it })} />
        </>
      ) : null}
    </div>
  );
};

const LabeledCheckbox = ({ label, checked, onCheck }: { label: string; checked: boolean; onCheck: (checked: boolean) => void }) => (
  <label className="flex items-center gap-3 py-3 pl-3">
    <Checkbox checked={checked} onCheckedChange={onCheck} />
    <Text typography="bodyMedium">{label}</Text>
  </label>
);

const CoordinateField = ({ label, value, onValue }: { label: string; value: string; onValue: (value: string) => void }) => (
  <OutlinedTextField value={value} onValueChange={onValue} label={label} singleLine isError={toDoubleOrNull(value) === null} keyboardType="decimal" className="w-full" />
);

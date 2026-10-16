import { useMemo } from 'react';
import { type Json, isNull, jsonObjects, optBoolean, optJSONArray, optText } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { format } from '@/kotlin/Std';
import type { DrawableName } from '@/res/drawables';
import type { SummaryLine } from './SetupRow';
import { SETUP, type SetupReadiness } from './SetupView';

export const SETUP_PARAMETERS_PAGE = 'Parameters';

export const SETUP_OVERVIEW_PAGE = '';

export type SetupComponent = {
  index: number;
  name: string;
  known: string | null;
  className: string;
  needsAttention: boolean;
  blockedReason: string | null;
  prerequisite: string | null;
};

export const SetupComponent = (fields: {
  index: number;
  name: string;
  needsAttention: boolean;
  known?: string | null;
  className?: string;
  blockedReason?: string | null;
  prerequisite?: string | null;
}): SetupComponent => ({ known: null, className: '', blockedReason: null, prerequisite: null, ...fields });

const COMPONENT_ICONS: readonly (readonly [string, DrawableName])[] = [
  ['Failsafe', 'ic_warning'],
  ['Airframe', 'ic_flight'],
  ['SubFrame', 'ic_flight'],
  ['Gimbal', 'ic_photo_camera'],
  ['Logging', 'ic_description'],
  ['Motor', 'ic_speed'],
  ['RemoteSupport', 'ic_link'],
  ['Scripting', 'ic_terminal'],
  ['Tuning', 'ic_tune'],
  ['Actuator', 'ic_speed'],
  ['Servo', 'ic_tune'],
  ['Follow', 'ic_my_location'],
  ['Airspeed', 'ic_speed'],
  ['Lights', 'ic_bolt'],
  ['ESP8266', 'ic_wifi'],
  ['Syslink', 'ic_wifi'],
];

export const setupIcon = (known: string | null, className: string = ''): DrawableName =>
  known === 'radio' || known === 'joystick'
    ? 'ic_gamepad'
    : known === 'flightModes'
      ? 'ic_toggle_on'
      : known === 'sensors'
        ? 'ic_sensors'
        : known === 'safety'
          ? 'ic_shield'
          : known === 'power'
            ? 'ic_bolt'
            : known === 'esc'
              ? 'ic_tune'
              : (COMPONENT_ICONS.find(([token]) => className.includes(token))?.[1] ?? 'ic_build');

export const setupSubtitle = (vehicle: string, firmware: string): string => [firmware, vehicle].filter((it) => it.trim() !== '').join(' · ');

export const readinessNote = (readiness: SetupReadiness | null, listed: boolean): string | null => {
  if (readiness === null || readiness.setupComplete === true || listed) return null;
  const note = [readiness.headline, readiness.detail].filter((it) => it.trim() !== '').join('. ');
  return note.trim() === '' ? null : note;
};

export const parameterCountText = (count: number): string | null => (count > 0 ? format('%,d parameters', count) : null);

export const attentionAction = (className: string): string => (['Sensors', 'Radio'].some((it) => className.includes(it)) ? 'Calibrate' : 'Set up');

const textOrNull = (json: Json, key: string): string | null => {
  const text = optText(json, key);
  return isNull(json, key) || text.trim() === '' ? null : text;
};

export const setupComponents = (view: Json | null): readonly SetupComponent[] => {
  const listed = optJSONArray(view, 'components');
  if (listed === null) return [];
  return listed.flatMap((element, index) => {
    if (typeof element !== 'object' || element === null || Array.isArray(element)) return [];
    const json = element as Json;
    const name = optText(json, 'name');
    return name.trim() === ''
      ? []
      : [
          SetupComponent({
            index,
            name,
            known: textOrNull(json, 'known'),
            className: optText(json, 'className'),
            needsAttention: optBoolean(json, 'needsAttention'),
            blockedReason: textOrNull(json, 'blockedReason'),
            prerequisite: textOrNull(json, 'prerequisite'),
          }),
        ];
  });
};

export const setupSummaries = (view: Json | null): ReadonlyMap<string, readonly SummaryLine[]> => {
  const listed = optJSONArray(view, 'components');
  if (listed === null) return new Map();
  return new Map(
    jsonObjects(listed).map((component) => [
      optText(component, 'name'),
      jsonObjects(optJSONArray(component, 'rows')).map((it) => ({ label: optText(it, 'label'), value: optText(it, 'value'), warn: optBoolean(it, 'warn') })),
    ]),
  );
};

export const setupMatches = (name: string, search: string): boolean => search.trim() === '' || name.toLowerCase().includes(search.trim().toLowerCase());

export const splitDesktopOnly = (components: readonly SetupComponent[], opensHere: (component: SetupComponent) => boolean): readonly [SetupComponent[], SetupComponent[]] => [
  components.filter(opensHere),
  components.filter((it) => !opensHere(it)),
];

export const useHasSetupPage = (name: string): boolean => {
  const setupJson = useQgcPath(SETUP);
  return useMemo(() => setupComponents(setupJson).some((it) => it.name === name), [setupJson, name]);
};

export const SetupScreen = (_props: { className?: string }) => null;

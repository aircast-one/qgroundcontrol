import { useRef, useState } from 'react';
import { type Json, arrayJSONObject, has, isNull, jsonObjects, opt, optArrayText, optBoolean, optDouble, optInt, optJSONArray, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useKeyedState, useTicker } from '@/kotlin/Compose';
import { coerceIn } from '@/kotlin/Std';
import { DropdownMenu, DropdownMenuItem, OutlinedButton, OutlinedTextField, Slider, Switch, Text } from '@/m3';
import { toDoubleOrNull } from '@/map/AltitudeEntry';
import { md } from '@/theme/Colors';

export const CAMERA_SETTINGS_VIEW = 'view.cameraSettings';
export const CAMERA_SETTING_SET = 'cameraSettings.set';
const CAMERA_SETTINGS_POLL_MS = 1000;

export type CameraOption = { label: string; value: unknown };

export type CameraSettingControl =
  | { kind: 'Toggle'; on: boolean }
  | { kind: 'Choice'; options: readonly CameraOption[]; selected: number }
  | { kind: 'Range'; min: number; max: number; step: number; value: number }
  | { kind: 'Entry'; text: string };

export const CameraSettingControl = {
  Toggle: (on: boolean): CameraSettingControl => ({ kind: 'Toggle', on }),
  Choice: (options: readonly CameraOption[], selected: number): CameraSettingControl => ({ kind: 'Choice', options, selected }),
  Range: (min: number, max: number, step: number, value: number): CameraSettingControl => ({ kind: 'Range', min, max, step, value }),
  Entry: (text: string): CameraSettingControl => ({ kind: 'Entry', text }),
};

export type CameraSetting = { name: string; label: string; readOnly: boolean; control: CameraSettingControl };

const number = (json: Json, key: string): number | null => {
  if (isNull(json, key) || !has(json, key)) return null;
  const value = optDouble(json, key);
  return Number.isNaN(value) ? null : value;
};

const valueText = (value: unknown): string => (value === null || value === undefined ? '' : typeof value === 'object' ? JSON.stringify(value) : String(value));

export const cameraSettingControl = (parameter: Json): CameraSettingControl => {
  const options = optJSONArray(parameter, 'options') ?? [];
  const listed = options.flatMap((_, at) => {
    const option = arrayJSONObject(options, at);
    return option === null ? [] : [{ label: optText(option, 'label'), value: opt(option, 'value') }];
  });
  const value = opt(parameter, 'value');
  const step = number(parameter, 'step');
  const min = number(parameter, 'min');
  const max = number(parameter, 'max');
  if (optBoolean(parameter, 'isBool')) return CameraSettingControl.Toggle(value === true || (typeof value === 'number' && Math.trunc(value) === 1));
  if (listed.length > 0) return CameraSettingControl.Choice(listed, optInt(parameter, 'selected', -1));
  if (step !== null && min !== null && max !== null) return CameraSettingControl.Range(min, max, step, typeof value === 'number' ? value : min);
  return CameraSettingControl.Entry(valueText(value));
};

export const cameraSettings = (view: Json | null): readonly CameraSetting[] => {
  if (optText(view, 'state') !== 'ready') return [];
  const parameters = optJSONArray(view, 'parameters');
  if (parameters === null) return [];
  const byName = new Map(jsonObjects(parameters).map((it) => [optText(it, 'name'), it] as const));
  const active = optJSONArray(view, 'activeSettings');
  if (active === null) return [];
  return active.flatMap((_, at) => {
    const p = byName.get(optArrayText(active, at));
    return p === undefined ? [] : [{ name: optText(p, 'name'), label: optText(p, 'description'), readOnly: optBoolean(p, 'readOnly'), control: cameraSettingControl(p) }];
  });
};

export const CameraDefinitionSettings = () => {
  const [settings, setSettings] = useState<readonly CameraSetting[]>([]);
  const [refusal, setRefusal] = useState<string | null>(null);
  useTicker(CAMERA_SETTINGS_POLL_MS, [], async () => setSettings(cameraSettings(await Qgc.get(CAMERA_SETTINGS_VIEW))));
  const write = (name: string, value: unknown) => void (async () => setRefusal(await Qgc.refusalOf(CAMERA_SETTING_SET, name, value)))();
  return (
    <div className="flex flex-col gap-1.5 px-5">
      {settings.map((setting) => (
        <CameraSettingRow key={setting.name} setting={setting} onWrite={(value) => write(setting.name, value)} />
      ))}
      {refusal !== null ? (
        <Text typography="bodySmall" color={md('error')}>
          {refusal}
        </Text>
      ) : null}
    </div>
  );
};

const CameraSettingRow = ({ setting, onWrite }: { setting: CameraSetting; onWrite: (value: unknown) => void }) => {
  const enabled = !setting.readOnly;
  const control = setting.control;
  return (
    <div className="flex w-full items-center">
      <Text typography="bodyMedium" className="flex-1">
        {setting.label}
      </Text>
      {control.kind === 'Toggle' ? (
        <Switch checked={control.on} enabled={enabled} label={setting.label} onCheckedChange={(on) => onWrite(on ? 1 : 0)} />
      ) : control.kind === 'Choice' ? (
        <ChoiceControl options={control.options} selected={control.selected} enabled={enabled} onWrite={onWrite} />
      ) : control.kind === 'Range' ? (
        <RangeControl control={control} enabled={enabled} label={setting.label} onWrite={onWrite} />
      ) : (
        <EntryControl text={control.text} enabled={enabled} onWrite={onWrite} />
      )}
    </div>
  );
};

const ChoiceControl = ({ options, selected, enabled, onWrite }: { options: readonly CameraOption[]; selected: number; enabled: boolean; onWrite: (value: unknown) => void }) => {
  const [open, setOpen] = useState(false);
  return (
    <div className="relative">
      <OutlinedButton enabled={enabled} onClick={() => setOpen(true)}>
        {options[selected]?.label ?? ''}
      </OutlinedButton>
      <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
        {options.map((option, at) => (
          <DropdownMenuItem
            key={at}
            text={option.label}
            onClick={() => {
              setOpen(false);
              onWrite(option.value);
            }}
          />
        ))}
      </DropdownMenu>
    </div>
  );
};

const RangeControl = ({ control, enabled, label, onWrite }: { control: Extract<CameraSettingControl, { kind: 'Range' }>; enabled: boolean; label: string; onWrite: (value: unknown) => void }) => {
  const [dragged, setDragged] = useKeyedState([control.value], () => control.value);
  const latest = useRef(dragged);
  latest.current = dragged;
  const steps = Math.trunc((control.max - control.min) / control.step) - 1;
  return (
    <div className="w-[180px]">
      <Slider
        value={dragged}
        onValueChange={(next) => {
          latest.current = next;
          setDragged(next);
        }}
        onValueChangeFinished={() => onWrite(latest.current)}
        valueRange={[control.min, control.max]}
        steps={coerceIn(steps, 0, 1000)}
        enabled={enabled}
        label={label}
      />
    </div>
  );
};

const EntryControl = ({ text, enabled, onWrite }: { text: string; enabled: boolean; onWrite: (value: unknown) => void }) => {
  const [typed, setTyped] = useKeyedState([text], () => text);
  return (
    <div className="w-[180px]">
      <OutlinedTextField value={typed} onValueChange={setTyped} singleLine enabled={enabled} onDone={() => onWrite(toDoubleOrNull(typed) ?? typed)} />
    </div>
  );
};

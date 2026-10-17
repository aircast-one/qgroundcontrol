import { useMemo, useState } from 'react';
import { type Json, jsonObjects, optBoolean, optInt, optJSONArray, optLong, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { AlertDialog, Button, DropdownMenu, DropdownMenuItem, OutlinedButton, RadioButton, Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';
import { useKeyedState } from '@/kotlin/Compose';

export const PX4_AIRFRAME_VIEW = 'view.px4Airframe';
export const PX4_AIRFRAME_SCREEN = 'px4Airframe';
export const PX4_AIRFRAME_APPLY = 'px4Airframe.apply';
export const PX4_AIRFRAME_RESET = 'px4Airframe.reset';

export type AirframeChoice = { name: string; autostartId: number };
export type AirframeGroup = { name: string; airframes: readonly AirframeChoice[]; image: string };

const AIRFRAME_IMAGES = import.meta.glob<string>('../../../src/AutoPilotPlugins/Common/Images/*.svg', { query: '?url', import: 'default', eager: true });

export const airframeImageAsset = (image: string): string | null => {
  if (image.trim() === '') return null;
  const file = `/${image.endsWith('.svg') ? image.slice(0, -'.svg'.length) : image}.svg`;
  return Object.entries(AIRFRAME_IMAGES).find(([path]) => path.endsWith(file))?.[1] ?? null;
};

export type Px4Airframes = {
  autostartId: number;
  custom: boolean;
  customText: string;
  heading: string;
  currentType: string | null;
  currentIndex: number;
  applyTitle: string;
  applyText: string;
  groups: readonly AirframeGroup[];
};

export const px4Airframes = (view: Json | null): Px4Airframes | null => {
  if (view === null || !optBoolean(view, 'available')) return null;
  const currentType = optText(view, 'currentType');
  return {
    autostartId: optLong(view, 'autostartId'),
    custom: optBoolean(view, 'custom'),
    customText: optText(view, 'customText'),
    heading: optText(view, 'heading'),
    currentType: currentType !== '' ? currentType : null,
    currentIndex: optInt(view, 'currentIndex'),
    applyTitle: optText(view, 'applyTitle'),
    applyText: optText(view, 'applyText').replaceAll('<br>', '\n'),
    groups: jsonObjects(optJSONArray(view, 'types')).map((type) => ({
      name: optText(type, 'name'),
      image: optText(type, 'image'),
      airframes: jsonObjects(optJSONArray(type, 'airframes')).map((it) => ({ name: optText(it, 'name'), autostartId: optLong(it, 'autostartId') })),
    })),
  };
};

export type AirframeSelection = { group: string; index: number };

export const initialSelection = (read: Px4Airframes): AirframeSelection | null => (read.currentType === null ? null : { group: read.currentType, index: read.currentIndex });

export const Px4AirframeScreen = ({ className }: { className?: string }) => {
  const view = useQgcPath(PX4_AIRFRAME_VIEW);
  const read = useMemo(() => px4Airframes(view), [view]);
  const [refusal, setRefusal] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [selection, setSelection] = useKeyedState<AirframeSelection | null>([read?.autostartId ?? null], () => (read === null ? null : initialSelection(read)));

  if (read === null) {
    return <Text className={`block p-4 ${className ?? ''}`}>This vehicle has no SYS_AUTOSTART and SYS_AUTOCONFIG.</Text>;
  }

  const act = (path: string, ...args: unknown[]) =>
    void (async () => {
      setRefusal(await Qgc.refusalOf(path, ...args));
    })();

  if (read.custom) {
    return (
      <div className={`flex h-full flex-col gap-3 p-4 ${className ?? ''}`}>
        <Text>{read.customText}</Text>
        <Button onClick={() => act(PX4_AIRFRAME_RESET)} className="self-center">
          Reset
        </Button>
        {refusal !== null ? <Text color={md('error')}>{refusal}</Text> : null}
      </div>
    );
  }

  const chosen = selection === null ? null : (read.groups.find((it) => it.name === selection.group)?.airframes[selection.index] ?? null);

  return (
    <div className={`flex h-full flex-col ${className ?? ''}`}>
      {confirming && chosen !== null ? (
        <AlertDialog
          onDismissRequest={() => setConfirming(false)}
          title={read.applyTitle}
          text={<span className="whitespace-pre-line">{read.applyText}</span>}
          confirmButton={
            <TextButton
              onClick={() => {
                setConfirming(false);
                act(PX4_AIRFRAME_APPLY, chosen.autostartId);
              }}
            >
              Apply
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setConfirming(false)}>Cancel</TextButton>}
        />
      ) : null}
      <div className="flex w-full items-center gap-3 p-4">
        <Text typography="bodyMedium" className="flex-1">
          {read.heading}
        </Text>
        <Button onClick={() => setConfirming(true)} enabled={chosen !== null}>
          {read.applyTitle}
        </Button>
      </div>
      {refusal !== null ? (
        <Text color={md('error')} className="block px-4">
          {refusal}
        </Text>
      ) : null}
      <div className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-4">
        {read.groups.map((group) => {
          const picked = selection?.group === group.name;
          return (
            <AirframeGroupCard
              key={group.name}
              group={group}
              picked={picked}
              index={picked ? (selection?.index ?? 0) : 0}
              onPick={(index) => setSelection({ group: group.name, index })}
            />
          );
        })}
      </div>
    </div>
  );
};

const AirframeGroupCard = ({ group, picked, index, onPick }: { group: AirframeGroup; picked: boolean; index: number; onPick: (index: number) => void }) => {
  const [open, setOpen] = useState(false);
  const asset = airframeImageAsset(group.image);
  return (
    <div
      className="w-full cursor-pointer rounded-shape-md"
      style={{ border: `1px solid ${picked ? md('primary') : md('outlineVariant')}`, background: picked ? md('secondaryContainer') : md('surface') }}
      onClick={() => onPick(index)}
    >
      <div className="flex items-center gap-3 p-3">
        <RadioButton selected={picked} onClick={() => onPick(index)} label={group.name} />
        <div className="flex min-w-0 flex-1 flex-col">
          <Text typography="titleSmall">{group.name}</Text>
          {asset !== null ? <img src={asset} alt={group.name} className="h-24 w-full object-contain py-1" /> : null}
          <div className="relative self-start" onClick={(event) => event.stopPropagation()}>
            <OutlinedButton onClick={() => setOpen(true)}>{group.airframes[index]?.name ?? ''}</OutlinedButton>
            <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
              {group.airframes.map((frame, at) => (
                <DropdownMenuItem
                  key={at}
                  text={frame.name}
                  onClick={() => {
                    setOpen(false);
                    onPick(at);
                  }}
                />
              ))}
            </DropdownMenu>
          </div>
        </div>
      </div>
    </div>
  );
};

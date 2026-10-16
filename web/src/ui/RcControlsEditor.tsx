import { Fragment, useMemo, useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { settingControl, useQgcDouble, useQgcString } from '@/bridge/QgcState';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { AlertDialog, Button, FilterChip, HorizontalDivider, OutlinedTextField, Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';
import {
  RC_CHANNEL_MAX,
  RC_CHANNEL_MIN,
  RcControlType,
  RcControlTypeEntries,
  channelOwner,
  channelUsable,
  firstFreeChannel,
  parseRcControls,
  rcControlsAdded,
  rcControlsPatched,
  rcControlsRemoved,
  typeLabel,
} from '@/ui/RcControls';
import { SectionHeader } from '@/ui/SetupRow';

const RC_CONTROLS = 'settings.flyViewSettings.rcControls';

const CAMERA_CHANNEL_SETTINGS: readonly (readonly [string, string])[] = [
  ['gimbalTiltChannel', 'Gimbal tilt'],
  ['gimbalPanChannel', 'Gimbal pan'],
  ['cameraZoomChannel', 'Camera zoom'],
  ['cameraLightChannel', 'Camera light'],
  ['cameraRecordChannel', 'Camera record'],
];

const UNDO_WINDOW_MS = 6000;

type Draft = { index: number; label: string; channel: string; type: RcControlType };

type Undo = { name: string; previous: string };

const channelOf = (value: number): number => (Number.isNaN(value) ? 0 : Math.trunc(value));

const useReservedChannels = (): ReadonlyMap<number, string> => {
  const channels = CAMERA_CHANNEL_SETTINGS.map(([name]) => channelOf(useQgcDouble(settingControl(`settings.flyViewSettings.${name}`), 0)));
  const key = channels.join(',');
  return useMemo(() => new Map(CAMERA_CHANNEL_SETTINGS.map(([, owner], at): [number, string] => [channels[at] ?? 0, owner]).filter(([channel]) => channel > 0)), [key]);
};

export const RcControlsEditor = ({ className }: { className?: string }) => {
  const json = useQgcString(settingControl(RC_CONTROLS));
  const controls = useMemo(() => parseRcControls(json), [json]);
  const reserved = useReservedChannels();
  const [draft, setDraft] = useState<Draft | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [undo, setUndo] = useState<Undo | null>(null);

  useLaunchedEffect([undo], async (scope) => {
    if (undo === null) return;
    await delay(UNDO_WINDOW_MS);
    if (scope.isActive()) setUndo(null);
  });

  const save = (next: string) => {
    setNotice(null);
    void (async () => {
      if (!(await Qgc.set(RC_CONTROLS, next))) setNotice('The setting would not take that list.');
    })();
  };

  const channel = draft === null ? 0 : Number.parseInt(draft.channel, 10) || 0;
  const owner = draft === null ? null : channelOwner(json, channel, draft.index, reserved);

  return (
    <>
      <div className={`flex flex-col ${className ?? ''}`}>
        <SectionHeader text="On-screen RC controls" />
        {notice !== null ? (
          <Text typography="bodyMedium" color={md('error')} className="block p-4">
            {notice}
          </Text>
        ) : null}

        {controls.length === 0 ? <Text className="block p-4">No on-screen controls yet. Add one to drive a channel from the Fly view.</Text> : null}

        {controls.map((control, index) => {
          const clash = channelOwner(json, control.channel, index, reserved);
          return (
            <Fragment key={index}>
              <div className="flex w-full items-center px-4 py-2.5">
                <div className="flex min-w-0 flex-1 flex-col">
                  <Text typography="bodyLarge">{control.label}</Text>
                  <Text typography="bodySmall">{`Channel ${control.channel} · ${typeLabel(control.type)}`}</Text>
                  {clash !== null ? (
                    <Text typography="bodySmall" color={md('error')}>
                      {`Channel ${control.channel} is already driving ${clash}.`}
                    </Text>
                  ) : null}
                </div>
                <TextButton onClick={() => setDraft({ index, label: control.label, channel: String(control.channel), type: control.type })}>Edit</TextButton>
                <TextButton
                  onClick={() => {
                    setUndo({ name: control.label, previous: json });
                    save(rcControlsRemoved(json, index));
                  }}
                >
                  Remove
                </TextButton>
              </div>
              <HorizontalDivider />
            </Fragment>
          );
        })}

        {undo !== null ? (
          <div className="flex w-full items-center px-4 py-1">
            <Text typography="bodyMedium" className="flex-1">{`Removed ${undo.name}.`}</Text>
            <TextButton
              onClick={() => {
                save(undo.previous);
                setUndo(null);
              }}
            >
              Undo
            </TextButton>
          </div>
        ) : null}

        <Button onClick={() => setDraft({ index: -1, label: '', channel: String(firstFreeChannel(json, reserved)), type: RcControlType.Slider })} className="m-4 self-start">
          Add control
        </Button>
      </div>

      {draft !== null ? (
        <AlertDialog
          onDismissRequest={() => setDraft(null)}
          title={draft.index < 0 ? 'New control' : 'Edit control'}
          text={
            <div className="flex flex-col gap-3">
              <OutlinedTextField value={draft.label} onValueChange={(it) => setDraft({ ...draft, label: it })} label="Name" />
              <OutlinedTextField value={draft.channel} onValueChange={(it) => setDraft({ ...draft, channel: it.replace(/\D/g, '').slice(0, 2) })} label="Channel" keyboardType="number" />
              {!channelUsable(channel) ? (
                <Text typography="bodySmall" color={md('error')}>
                  {`Channels run from ${RC_CHANNEL_MIN} to ${RC_CHANNEL_MAX}.`}
                </Text>
              ) : null}
              {owner !== null ? (
                <Text typography="bodySmall" color={md('error')}>
                  {`Channel ${channel} is already driving ${owner}.`}
                </Text>
              ) : null}
              <div className="flex flex-wrap gap-2">
                {RcControlTypeEntries.map((type) => (
                  <FilterChip key={type.name} selected={type === draft.type} onClick={() => setDraft({ ...draft, type })} label={typeLabel(type)} />
                ))}
              </div>
            </div>
          }
          confirmButton={
            <TextButton
              enabled={channelUsable(channel) && owner === null}
              onClick={() => {
                const label = draft.label.trim() === '' ? `CH${channel}` : draft.label;
                save(draft.index < 0 ? rcControlsAdded(json, label, channel, draft.type) : rcControlsPatched(json, draft.index, label, channel, draft.type));
                setDraft(null);
              }}
            >
              Save
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setDraft(null)}>Cancel</TextButton>}
        />
      ) : null}
    </>
  );
};

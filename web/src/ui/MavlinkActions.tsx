import { useState } from 'react';
import { type Json, jsonObjects, jsonStrings, optJSONArray, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { DropdownMenu, DropdownMenuItem, OutlinedButton, Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';

export const MAVLINK_ACTIONS_VIEW = 'view.mavlinkActions';
export const MAVLINK_ACTIONS_SEND = 'mavlinkActions.send';
export const MAVLINK_ACTIONS_GROUP = 'mavlinkActionsSettings';
export const NO_ACTIONS_FILE = '<None>';

export type MavlinkActionEntry = { label: string; description: string };

export type MavlinkActions = {
  files: readonly string[];
  flyViewFile: string;
  joystickFile: string;
  flyViewPath: string;
  joystickPath: string;
  actions: readonly MavlinkActionEntry[];
  folderNote: string;
};

export const mavlinkActions = (view: Json | null): MavlinkActions | null => {
  const files = optJSONArray(view, 'files');
  if (view === null || files === null) return null;
  return {
    files: jsonStrings(files),
    flyViewFile: optText(view, 'flyViewFile'),
    joystickFile: optText(view, 'joystickFile'),
    flyViewPath: optText(view, 'flyViewPath'),
    joystickPath: optText(view, 'joystickPath'),
    actions: jsonObjects(optJSONArray(view, 'actions')).map((it) => ({ label: optText(it, 'label'), description: optText(it, 'description') })),
    folderNote: optText(view, 'folderNote'),
  };
};

export const chosenFile = (option: string): string => (option === NO_ACTIONS_FILE ? '' : option);

export const MavlinkActionsSection = ({ onWrite }: { onWrite: () => void }) => {
  const [revision, setRevision] = useState(0);
  const [read, setRead] = useState<MavlinkActions | null>(null);
  useLaunchedEffect([revision], async (scope) => {
    const next = mavlinkActions(await Qgc.get(MAVLINK_ACTIONS_VIEW));
    if (scope.isActive()) setRead(next);
  });
  const actions = read;
  if (actions === null) return null;
  const options = [NO_ACTIONS_FILE, ...actions.files];
  const choose = async (path: string, option: string) => {
    await Qgc.set(path, chosenFile(option));
    setRevision((it) => it + 1);
    onWrite();
  };
  return (
    <div className="flex w-full flex-col px-5 py-2">
      {actions.folderNote.trim() !== '' ? (
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          {actions.folderNote}
        </Text>
      ) : null}
      <FileChoice label="Fly view actions" current={actions.flyViewFile} options={options} onPick={(it) => void choose(actions.flyViewPath, it)} />
      <FileChoice label="Joystick actions" current={actions.joystickFile} options={options} onPick={(it) => void choose(actions.joystickPath, it)} />
    </div>
  );
};

const FileChoice = ({ label, current, options, onPick }: { label: string; current: string; options: readonly string[]; onPick: (option: string) => void }) => {
  const [open, setOpen] = useState(false);
  return (
    <div className="flex w-full items-center py-1">
      <Text className="flex-1">{label}</Text>
      <div className="relative">
        <OutlinedButton enabled={options.length > 1} onClick={() => setOpen(true)}>
          {current.trim() === '' ? NO_ACTIONS_FILE : current}
        </OutlinedButton>
        <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
          {options.map((option) => (
            <DropdownMenuItem
              key={option}
              text={option}
              onClick={() => {
                setOpen(false);
                onPick(option);
              }}
            />
          ))}
        </DropdownMenu>
      </div>
    </div>
  );
};

export const FlyViewMavlinkActions = ({ onSent }: { onSent: () => void }) => {
  const [read, setRead] = useState<MavlinkActions | null>(null);
  useLaunchedEffect([], async (scope) => {
    const next = mavlinkActions(await Qgc.get(MAVLINK_ACTIONS_VIEW));
    if (scope.isActive()) setRead(next);
  });
  return (
    <>
      {(read?.actions ?? []).map((action, index) => (
        <TextButton
          key={index}
          onClick={() => {
            onSent();
            void Qgc.invoke(MAVLINK_ACTIONS_SEND, index);
          }}
          className="w-full"
        >
          <Text fontWeight="bold" color={md('primary')} className="w-full text-left">
            {action.label}
          </Text>
        </TextButton>
      ))}
    </>
  );
};

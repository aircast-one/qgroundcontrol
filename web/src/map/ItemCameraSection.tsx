import { useState } from 'react';
import type { Json } from '@/bridge/Json';
import { useProduceState } from '@/kotlin/Compose';
import { DropdownMenu, DropdownMenuItem, OutlinedTextField, Text } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { CameraSectionExtras } from './CameraSectionExtras';
import { ItemCameraBridge, cameraChoices, cameraExtras, itemCameraNote, itemCameraTextBeside } from './ItemCamera';
import { useKeyedState } from './SettingStepper';

export const ItemCameraSection = ({ index, className, onChanged = () => undefined }: { index: number; className?: string; onChanged?: () => void }) => {
  const [revision, setRevision] = useState(0);
  const [open, setOpen] = useKeyedState([index], () => false);
  const camera = useProduceState<Json | null>(null, [index, revision], async (set) => set(await ItemCameraBridge.read(index)));
  const choices = cameraChoices(camera);
  if (choices === null) return null;
  const write = (work: () => Promise<boolean>) => {
    void (async () => {
      await work();
      setRevision((it) => it + 1);
      onChanged();
    })();
  };
  const beside = itemCameraTextBeside(camera, choices.labels[choices.chosen] ?? null);
  const extras = cameraExtras(camera);
  const note = itemCameraNote(camera);
  return (
    <div className={`flex flex-col ${className ?? ''}`}>
      {beside !== null ? <CameraNote text={beside} /> : null}
      <div className="relative w-full">
        <OutlinedTextField
          value={choices.labels[choices.chosen] ?? '…'}
          onValueChange={() => undefined}
          readOnly
          label="Camera"
          trailingIcon={<MaterialIcon name="arrow_drop_down" filled />}
          singleLine
          className="w-full"
        />
        <button type="button" aria-label="Camera" aria-haspopup="menu" aria-expanded={open} className="absolute inset-0" onClick={() => setOpen(true)} />
        <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
          {choices.labels.map((label, at) => (
            <DropdownMenuItem
              key={at}
              text={label}
              onClick={() => {
                setOpen(false);
                write(() => ItemCameraBridge.chooseAction(index, at));
              }}
            />
          ))}
        </DropdownMenu>
      </div>
      {extras !== null ? <CameraSectionExtras extras={extras} write={(member, value) => write(() => ItemCameraBridge.set(index, member, value))} /> : null}
      {note !== null ? <CameraNote text={note} /> : null}
    </div>
  );
};

const CameraNote = ({ text }: { text: string }) => (
  <Text typography="bodySmall" color={md('onSurfaceVariant')} className="block w-full py-1">
    {text}
  </Text>
);

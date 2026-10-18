import { type ReactNode, useState } from 'react';
import { AlertDialog, OutlinedTextField, Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';
import { levelColor } from '@/ui/DeviceKit';
import { DeviceLevel } from '@/ui/DeviceLevel';

export const TypedConfirmDialog = ({
  title,
  text,
  label,
  expected,
  confirm,
  onConfirm,
  onDismiss,
}: {
  title: string;
  text: string;
  label: string;
  expected: string;
  confirm: string;
  onConfirm: (typed: string) => void;
  onDismiss: () => void;
}) => {
  const [typed, setTyped] = useState('');
  const matches = typed.trim() === expected && expected !== '';
  return (
    <AlertDialog
      onDismissRequest={onDismiss}
      title={title}
      text={
        <div className="flex flex-col gap-4">
          <span>{text}</span>
          <OutlinedTextField label={label} ariaLabel={label} value={typed} placeholder={expected} autoCorrect={false} autoFocus onValueChange={setTyped} />
        </div>
      }
      confirmButton={
        <TextButton
          enabled={matches}
          onClick={() => {
            onDismiss();
            onConfirm(typed.trim());
          }}
        >
          {confirm}
        </TextButton>
      }
      dismissButton={<TextButton onClick={onDismiss}>Cancel</TextButton>}
    />
  );
};

export const StepText = ({ step, title, text = '' }: { step: number; title: string; text?: string }) => (
  <div className="flex flex-col px-list pt-3 pb-1">
    <Text typography="titleSmall">{`${step} · ${title}`}</Text>
    {text !== '' ? (
      <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
        {text}
      </Text>
    ) : null}
  </div>
);

export const StatusLine = ({ text, level = DeviceLevel.Neutral }: { text: string; level?: number }) =>
  text === '' ? null : (
    <div className="px-list py-2">
      <Text typography="bodyMedium" color={level === DeviceLevel.Neutral ? md('onSurfaceVariant') : levelColor(level)}>
        {text}
      </Text>
    </div>
  );

export const ButtonRow = ({ children }: { children: ReactNode }) => <div className="flex flex-wrap items-center gap-2 px-list py-2">{children}</div>;

export const StackedFact = ({ label, value }: { label: string; value: string }) => (
  <div className="flex flex-col px-list py-2">
    <Text typography="bodySmall" color={md('onSurfaceVariant')}>
      {label}
    </Text>
    <Text typography="bodyLarge" className="break-words">
      {value}
    </Text>
  </div>
);

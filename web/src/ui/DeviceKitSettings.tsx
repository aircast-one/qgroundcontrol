import { type ReactNode, useState } from 'react';
import { AlertDialog, IconButton, OutlinedTextField, Switch, Text, TextButton } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { type ActionConfirm, ConfirmDialog, type DeviceRequest, RefusalText, levelColor, useDeviceAction } from '@/ui/DeviceKit';
import { DeviceLevel } from '@/ui/DeviceStatus';

export const SecretField = ({
  label,
  value,
  placeholder,
  isError = false,
  supportingText,
  onValueChange,
}: {
  label: string;
  value: string;
  placeholder?: string;
  isError?: boolean;
  supportingText?: string;
  onValueChange: (value: string) => void;
}) => {
  const [shown, setShown] = useState(false);
  return (
    <OutlinedTextField
      label={label}
      ariaLabel={label}
      value={value}
      placeholder={placeholder}
      keyboardType={shown ? 'text' : 'password'}
      autoCorrect={false}
      isError={isError}
      supportingText={supportingText}
      onValueChange={onValueChange}
      trailingIcon={
        <IconButton label={shown ? `Hide ${label}` : `Show ${label}`} onClick={() => setShown(!shown)}>
          <MaterialIcon name={shown ? 'visibility_off' : 'visibility'} />
        </IconButton>
      }
    />
  );
};

const linkLook = 'inline-flex h-10 select-none items-center justify-center gap-2 rounded-full text-label-large no-underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--md-primary)]';

export const LinkButton = ({ href, children, external = false, download = false, tonal = true }: { href: string; children: ReactNode; external?: boolean; download?: boolean; tonal?: boolean }) => (
  <a
    href={href}
    target={external ? '_blank' : undefined}
    rel={external ? 'noopener noreferrer' : undefined}
    download={download ? '' : undefined}
    className={`${linkLook} ${tonal ? 'bg-secondary-container px-6 text-on-secondary-container hover:brightness-110' : 'px-3 text-primary hover:bg-[color-mix(in_srgb,var(--md-primary)_8%,transparent)]'}`}
  >
    {children}
    {external ? <MaterialIcon name="open_in_new" size={18} /> : null}
  </a>
);

export const SettingSwitchRow = ({
  title,
  detail = '',
  checked,
  on,
  off,
  onSuccess = '',
  offSuccess = '',
  enabled = true,
  confirmOn = null,
}: {
  title: string;
  detail?: string;
  checked: boolean;
  on: DeviceRequest;
  off: DeviceRequest;
  onSuccess?: string;
  offSuccess?: string;
  enabled?: boolean;
  confirmOn?: ActionConfirm | null;
}) => {
  const action = useDeviceAction();
  const [pending, setPending] = useState<boolean | null>(null);
  const [asking, setAsking] = useState(false);
  const flip = (next: boolean) => {
    setPending(next);
    void action.run(next ? on : off, next ? onSuccess : offSuccess).then(() => setPending(null));
  };
  return (
    <>
      <div className="flex items-center gap-4 px-list py-3">
        <div className="flex min-w-0 flex-1 flex-col">
          <Text typography="bodyLarge">{title}</Text>
          {detail !== '' ? (
            <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
              {detail}
            </Text>
          ) : null}
        </div>
        <Switch checked={pending ?? checked} enabled={enabled && !action.busy} label={title} onCheckedChange={(next) => (next && confirmOn !== null ? setAsking(true) : flip(next))} />
      </div>
      <RefusalText refusal={action.refusal} />
      {asking && confirmOn !== null ? <ConfirmDialog {...confirmOn} onConfirm={() => flip(true)} onDismiss={() => setAsking(false)} /> : null}
    </>
  );
};

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

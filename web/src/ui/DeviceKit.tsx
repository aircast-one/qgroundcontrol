import { type ReactNode, useContext, useEffect, useState } from 'react';
import { isJson, jsonObjects, optInt, optJSONArray, optText } from '@/bridge/Json';
import { copyToClipboard } from '@/kotlin/Platform';
import { AlertDialog, BasicAlertDialog, DropdownMenu, DropdownMenuItem, FilledTonalButton, IconButton, type KeyboardType, OutlinedTextField, Switch, Text, TextButton } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { type DeviceMethod, type DeviceRefusal, deviceCall, deviceQr } from '@/ui/DeviceView';
import { LocalChangeNotice } from '@/ui/ValueRow';

export type DeviceBadge = { readonly level: number; readonly text: string };

export type DeviceOption = { readonly value: string; readonly label: string };

export type DeviceChartView = { readonly label: string; readonly values: readonly number[]; readonly max: number; readonly value: string; readonly level: number };

export type DeviceRequest = { readonly method: DeviceMethod; readonly path: string; readonly body?: unknown };

export const parseBadge = (json: unknown): DeviceBadge => {
  const it = isJson(json) ? json : null;
  return { level: optInt(it, 'level'), text: optText(it, 'text') };
};

export const parseBadges = (json: unknown, key: string): readonly DeviceBadge[] => jsonObjects(optJSONArray(isJson(json) ? json : null, key)).map(parseBadge);

export const parseOptions = (json: unknown, key: string): readonly DeviceOption[] =>
  jsonObjects(optJSONArray(isJson(json) ? json : null, key)).map((option) => ({ value: optText(option, 'value'), label: optText(option, 'label') }));

export const parseChart = (json: unknown): DeviceChartView | null => {
  const it = isJson(json) ? json : null;
  return it === null
    ? null
    : {
        label: optText(it, 'label'),
        values: (optJSONArray(it, 'values') ?? []).filter((value): value is number => typeof value === 'number'),
        max: optInt(it, 'max', 100),
        value: optText(it, 'value'),
        level: optInt(it, 'level', DeviceLevel.Neutral),
      };
};

export const levelColor = (level: number): string =>
  level === DeviceLevel.Ready ? aircast('success') : level === DeviceLevel.Warn ? aircast('warning') : level === DeviceLevel.Blocked ? aircast('alert') : md('onSurfaceVariant');

const levelIcon = (level: number): string => (level === DeviceLevel.Ready ? 'check_circle' : level === DeviceLevel.Neutral ? 'info' : 'warning');

export const DeviceSection = ({ title, badge, children }: { title: string; badge?: DeviceBadge | null; children: ReactNode }) => (
  <section aria-label={title}>
    <div className="flex items-center gap-3 px-list pt-5 pb-2">
      <Text typography="labelLarge" color={md('onSurfaceVariant')} className="min-w-0 flex-1">
        {title}
      </Text>
      {badge ? <DeviceBadgeText badge={badge} /> : null}
    </div>
    {children}
  </section>
);

export const DeviceBadgeText = ({ badge }: { badge: DeviceBadge }) =>
  badge.text === '' ? null : (
    <span className="flex shrink-0 items-center gap-1.5">
      <span className="size-2 rounded-full" style={{ background: levelColor(badge.level) }} />
      <Text typography="labelMedium" color={levelColor(badge.level)}>
        {badge.text}
      </Text>
    </span>
  );

export const DeviceNotice = ({ notice }: { notice: DeviceBadge }) => (
  <div role={notice.level === DeviceLevel.Blocked ? 'alert' : 'note'} className="mx-list my-2 flex items-start gap-3 rounded-shape-md bg-surface-container-high px-4 py-3">
    <MaterialIcon name={levelIcon(notice.level)} size={20} style={{ color: levelColor(notice.level) }} />
    <Text typography="bodyMedium" className="min-w-0 flex-1">
      {notice.text}
    </Text>
  </div>
);

export const DeviceText = ({ text }: { text: string }) =>
  text === '' ? null : (
    <div className="px-list py-2">
      <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
        {text}
      </Text>
    </div>
  );

export const DeviceFact = ({ label, value, level = DeviceLevel.Neutral }: { label: string; value: string; level?: number }) => (
  <div className="flex items-baseline justify-between gap-4 px-list py-2">
    <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
      {label}
    </Text>
    <Text typography="bodyLarge" color={level === DeviceLevel.Neutral ? md('onSurface') : levelColor(level)} className="min-w-0 text-right break-words">
      {value}
    </Text>
  </div>
);

export const useCopy = (): ((text: string) => void) => {
  const notice = useContext(LocalChangeNotice);
  return (text) =>
    void copyToClipboard(text).then(
      () => notice.show('Copied', null),
      () => notice.show('Could not copy — select the text instead', null),
    );
};

export const QrImage = ({ text, alt, className }: { text: string; alt: string; className: string }) => {
  const [shown, setShown] = useState<{ readonly text: string; readonly source: string | null } | null>(null);
  useEffect(() => void deviceQr(text).then((source) => setShown({ text, source })), [text]);
  return shown?.text === text && shown.source !== null ? <img src={shown.source} alt={alt} className={className} /> : <span role="img" aria-label={alt} className={className} />;
};

export const CopyRow = ({ label, text, hint = '', qr = false, level = DeviceLevel.Neutral }: { label: string; text: string; hint?: string; qr?: boolean; level?: number }) => {
  const copy = useCopy();
  const [showingQr, setShowingQr] = useState(false);
  return (
    <div className="flex items-center gap-2 px-list py-2">
      <div className="flex min-w-0 flex-1 flex-col">
        <Text typography="bodyMedium" color={level === DeviceLevel.Neutral ? md('onSurfaceVariant') : levelColor(level)}>
          {label}
        </Text>
        <Text typography="bodyLarge" monospace className="break-all select-all">
          {text}
        </Text>
        {hint !== '' ? (
          <Text typography="bodySmall" color={md('onSurfaceVariant')}>
            {hint}
          </Text>
        ) : null}
      </div>
      {qr ? (
        <IconButton label={`Show ${label} as a QR code`} onClick={() => setShowingQr(true)}>
          <MaterialIcon name="qr_code_2" />
        </IconButton>
      ) : null}
      <IconButton label={`Copy ${label}`} onClick={() => copy(text)}>
        <MaterialIcon name="content_copy" />
      </IconButton>
      {showingQr ? <QrDialog title={label} text={text} onDismiss={() => setShowingQr(false)} /> : null}
    </div>
  );
};

export const QrDialog = ({ title, text, onDismiss }: { title: string; text: string; onDismiss: () => void }) => (
  <BasicAlertDialog onDismissRequest={onDismiss} label={title}>
    <div className="flex flex-col items-center gap-3 p-6">
      <Text typography="titleMedium">{title}</Text>
      <QrImage text={text} alt={`QR code for ${text}`} className="size-64 rounded-shape-sm bg-white p-2" />
      <Text typography="bodySmall" monospace className="max-w-64 break-all text-center">
        {text}
      </Text>
      <TextButton onClick={onDismiss}>Done</TextButton>
    </div>
  </BasicAlertDialog>
);

export type DeviceAction = {
  readonly busy: boolean;
  readonly refusal: DeviceRefusal | null;
  readonly run: (request: DeviceRequest, success?: string) => Promise<boolean>;
  readonly clear: () => void;
};

export const useDeviceAction = (): DeviceAction => {
  const notice = useContext(LocalChangeNotice);
  const [busy, setBusy] = useState(false);
  const [refusal, setRefusal] = useState<DeviceRefusal | null>(null);
  const run = async ({ method, path, body }: DeviceRequest, success = ''): Promise<boolean> => {
    setBusy(true);
    setRefusal(null);
    const refused = await deviceCall(method, path, body);
    setBusy(false);
    setRefusal(refused);
    if (refused === null && success !== '') notice.show(success, null);
    return refused === null;
  };
  return { busy, refusal, run, clear: () => setRefusal(null) };
};

export const RefusalText = ({ refusal, field = '' }: { refusal: DeviceRefusal | null; field?: string }) =>
  refusal === null || refusal.field !== field ? null : (
    <div role="alert" className="px-list py-1">
      <Text typography="bodySmall" color={md('error')}>
        {refusal.message}
      </Text>
    </div>
  );

export const ConfirmDialog = ({ title, text, confirm, onConfirm, onDismiss }: { title: string; text: string; confirm: string; onConfirm: () => void; onDismiss: () => void }) => (
  <AlertDialog
    onDismissRequest={onDismiss}
    title={title}
    text={text}
    confirmButton={
      <TextButton
        onClick={() => {
          onDismiss();
          onConfirm();
        }}
      >
        {confirm}
      </TextButton>
    }
    dismissButton={<TextButton onClick={onDismiss}>Cancel</TextButton>}
  />
);

export type ActionConfirm = { readonly title: string; readonly text: string; readonly confirm: string };

export const ActionButton = ({
  label,
  request,
  success = '',
  confirm = null,
  tonal = true,
  onDone = () => undefined,
}: {
  label: string;
  request: DeviceRequest;
  success?: string;
  confirm?: ActionConfirm | null;
  tonal?: boolean;
  onDone?: () => void;
}) => {
  const action = useDeviceAction();
  const [asking, setAsking] = useState(false);
  const go = () => void action.run(request, success).then((done) => (done ? onDone() : undefined));
  const Look = tonal ? FilledTonalButton : TextButton;
  return (
    <>
      <Look enabled={!action.busy} onClick={() => (confirm === null ? go() : setAsking(true))}>
        {label}
      </Look>
      {asking && confirm !== null ? <ConfirmDialog {...confirm} onConfirm={go} onDismiss={() => setAsking(false)} /> : null}
      <RefusalText refusal={action.refusal} />
    </>
  );
};

export const DeviceSwitchRow = ({
  title,
  detail = '',
  checked,
  on,
  off,
  success = '',
  offSuccess = success,
  enabled = true,
  confirmOn = null,
}: {
  title: string;
  detail?: string;
  checked: boolean;
  on: DeviceRequest;
  off: DeviceRequest;
  success?: string;
  offSuccess?: string;
  enabled?: boolean;
  confirmOn?: ActionConfirm | null;
}) => {
  const action = useDeviceAction();
  const [pending, setPending] = useState<boolean | null>(null);
  const [asking, setAsking] = useState(false);
  const flip = (next: boolean) => {
    setPending(next);
    void action.run(next ? on : off, next ? success : offSuccess).then(() => setPending(null));
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

export type LinkLook = 'filled' | 'tonal' | 'outlined' | 'text';

const LINK_LOOKS: Readonly<Record<LinkLook, string>> = {
  filled: 'bg-primary px-6 text-on-primary hover:brightness-110',
  tonal: 'bg-secondary-container px-6 text-on-secondary-container hover:brightness-110',
  outlined: 'border border-outline px-6 text-primary hover:bg-[color-mix(in_srgb,var(--md-primary)_8%,transparent)]',
  text: 'px-3 text-primary hover:bg-[color-mix(in_srgb,var(--md-primary)_8%,transparent)]',
};

export const LinkButton = ({
  href,
  children,
  look = 'tonal',
  external = false,
  download = false,
  title,
}: {
  href: string;
  children: ReactNode;
  look?: LinkLook;
  external?: boolean;
  download?: boolean;
  title?: string;
}) => (
  <a
    href={href}
    target={external ? '_blank' : undefined}
    rel={external ? 'noopener noreferrer' : undefined}
    download={download ? '' : undefined}
    title={title}
    className={`inline-flex h-10 select-none items-center justify-center gap-2 rounded-full text-label-large no-underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--md-primary)] ${LINK_LOOKS[look]}`}
  >
    {children}
    {external ? <MaterialIcon name="open_in_new" size={18} /> : null}
  </a>
);

export const DeviceSelect = ({ label, value, options, onPick, enabled = true }: { label: string; value: string; options: readonly DeviceOption[]; onPick: (value: string) => void; enabled?: boolean }) => {
  const [open, setOpen] = useState(false);
  const shown = options.find((option) => option.value === value)?.label ?? value;
  return (
    <div className="grid gap-label">
      <Text typography="bodySmall" color={md('onSurfaceVariant')}>
        {label}
      </Text>
      <div className="relative">
        <button
          type="button"
          aria-label={label}
          aria-haspopup="menu"
          disabled={!enabled}
          onClick={() => setOpen(true)}
          className="flex min-h-14 w-full items-center gap-3 rounded-shape-xs border border-outline px-4 text-left text-body-large text-on-surface disabled:opacity-40"
        >
          <span className="min-w-0 flex-1 truncate">{shown}</span>
          <MaterialIcon name="arrow_drop_down" />
        </button>
        <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
          {options.map((option) => (
            <DropdownMenuItem
              key={option.value}
              text={option.label}
              checked={option.value === value}
              onClick={() => {
                setOpen(false);
                onPick(option.value);
              }}
            />
          ))}
        </DropdownMenu>
      </div>
    </div>
  );
};

export type DeviceField = {
  readonly name: string;
  readonly label: string;
  readonly value: string;
  readonly placeholder?: string;
  readonly hint?: string;
  readonly keyboard?: KeyboardType;
  readonly options?: readonly DeviceOption[];
  readonly lines?: number;
};

export const formBody = (fields: readonly DeviceField[], values: Readonly<Record<string, string>>): Record<string, string> =>
  Object.fromEntries(fields.map((field) => [field.name, values[field.name] ?? field.value]));

export const DeviceForm = ({
  fields,
  method,
  path,
  submit,
  success = '',
  confirm = null,
  note = '',
  extra = {},
  onDone = () => undefined,
}: {
  fields: readonly DeviceField[];
  method: DeviceMethod;
  path: string;
  submit: string;
  success?: string;
  confirm?: ActionConfirm | null;
  note?: string;
  extra?: Readonly<Record<string, string>>;
  onDone?: () => void;
}) => {
  const action = useDeviceAction();
  const [values, setValues] = useState<Readonly<Record<string, string>>>({});
  const [asking, setAsking] = useState(false);
  const set = (name: string) => (value: string) => setValues((it) => ({ ...it, [name]: value }));
  const send = () =>
    void action.run({ method, path, body: { ...formBody(fields, values), ...extra } }, success).then((done) => {
      if (!done) return;
      setValues({});
      onDone();
    });
  return (
    <div className="flex flex-col gap-3 px-list py-2">
      {fields.map((field) => (
        <div key={field.name}>
          {field.options !== undefined ? (
            <DeviceSelect label={field.label} value={values[field.name] ?? field.value} options={field.options} onPick={set(field.name)} />
          ) : (
            <OutlinedTextField
              label={field.label}
              value={values[field.name] ?? field.value}
              placeholder={field.placeholder}
              keyboardType={field.keyboard ?? 'text'}
              singleLine={(field.lines ?? 1) === 1}
              minLines={field.lines}
              autoCorrect={false}
              isError={action.refusal?.field === field.name}
              supportingText={action.refusal?.field === field.name ? action.refusal.message : field.hint}
              onValueChange={set(field.name)}
            />
          )}
        </div>
      ))}
      {note !== '' ? (
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          {note}
        </Text>
      ) : null}
      {action.refusal !== null && !fields.some((field) => field.name === action.refusal?.field) ? (
        <Text typography="bodySmall" color={md('error')}>
          {action.refusal.message}
        </Text>
      ) : null}
      <div>
        <FilledTonalButton enabled={!action.busy} onClick={() => (confirm === null ? send() : setAsking(true))}>
          {submit}
        </FilledTonalButton>
      </div>
      {asking && confirm !== null ? <ConfirmDialog {...confirm} onConfirm={send} onDismiss={() => setAsking(false)} /> : null}
    </div>
  );
};

export const chartPoints = (values: readonly number[], max: number, width: number, height: number): string =>
  values.length < 2
    ? ''
    : values.map((value, index) => `${((index / (values.length - 1)) * width).toFixed(1)},${(height - (Math.min(Math.max(value, 0), max) / (max > 0 ? max : 1)) * height).toFixed(1)}`).join(' ');

const CHART_WIDTH = 240;
const CHART_HEIGHT = 40;

export const DeviceChart = ({ chart }: { chart: DeviceChartView }) => (
  <div className="flex items-center gap-4 px-list py-2">
    <div className="flex w-28 shrink-0 flex-col">
      <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
        {chart.label}
      </Text>
      <Text typography="titleMedium" color={chart.level === DeviceLevel.Neutral ? md('onSurface') : levelColor(chart.level)}>
        {chart.value}
      </Text>
    </div>
    <svg viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`} preserveAspectRatio="none" className="h-10 min-w-0 flex-1" role="img" aria-label={`${chart.label} history`}>
      <polyline points={chartPoints(chart.values, chart.max, CHART_WIDTH, CHART_HEIGHT)} fill="none" stroke={chart.level === DeviceLevel.Neutral ? md('primary') : levelColor(chart.level)} strokeWidth="2" vectorEffect="non-scaling-stroke" />
    </svg>
  </div>
);

export const DeviceDisclosure = ({
  title,
  summary = '',
  badge = null,
  initiallyOpen = false,
  children,
}: {
  title: string;
  summary?: string;
  badge?: DeviceBadge | null;
  initiallyOpen?: boolean;
  children: ReactNode;
}) => {
  const [open, setOpen] = useState(initiallyOpen);
  return (
    <div>
      <button type="button" aria-expanded={open} onClick={() => setOpen(!open)} className="flex min-h-14 w-full items-center gap-4 px-list py-2 text-left hover:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)]">
        <span className="flex min-w-0 flex-1 flex-col">
          <Text typography="bodyLarge">{title}</Text>
          {summary !== '' ? (
            <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
              {summary}
            </Text>
          ) : null}
        </span>
        {badge !== null ? <DeviceBadgeText badge={badge} /> : null}
        <MaterialIcon name={open ? 'expand_less' : 'expand_more'} style={{ color: md('onSurfaceVariant') }} />
      </button>
      {open ? children : null}
    </div>
  );
};

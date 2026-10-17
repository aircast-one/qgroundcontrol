import { type KeyboardEvent, type ReactNode, Fragment, createContext, useContext, useEffect } from 'react';
import { IconButton, Text, TextButton } from '@/m3';
import type { KeyboardType } from '@/m3';
import { type MutableState } from '@/kotlin/State';
import { useUpdatedRef } from '@/kotlin/Compose';
import { replaceFirstCharLower } from '@/kotlin/Std';
import { sentenceCase } from '@/map/SentenceCase';
import { type DrawableName } from '@/res/drawables';
import { Drawable, MaterialIcon } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';
import { LocalSettingsList } from '@/ui/SettingsScreen';

export type SetupState = { readonly name: 'NeedsAttention' | 'Done' | 'Neutral' | 'Unavailable'; readonly ordinal: number };

export const SetupState = {
  NeedsAttention: { name: 'NeedsAttention', ordinal: 0 },
  Done: { name: 'Done', ordinal: 1 },
  Neutral: { name: 'Neutral', ordinal: 2 },
  Unavailable: { name: 'Unavailable', ordinal: 3 },
} as const satisfies Record<string, SetupState>;

export const SetupStateEntries: readonly SetupState[] = [SetupState.NeedsAttention, SetupState.Done, SetupState.Neutral, SetupState.Unavailable];

export const useSetupStateColor = (state: SetupState): string => {
  if (state === SetupState.NeedsAttention) return aircast('warning');
  if (state === SetupState.Done) return aircast('success');
  return md('onSurfaceVariant');
};

const SECTION_IMAGES = {
  ...import.meta.glob<string>('../../../src/Toolbar/Images/{Battery,Gears}.svg', { query: '?url', import: 'default', eager: true }),
  ...import.meta.glob<string>(
    '../../../src/AutoPilotPlugins/PX4/Images/{DatalinkLoss,GeoFence,LandModeCopter,LowBattery,ObjectAvoidance,RCLoss,ReturnToHomeAltitude,ReturnToHomeAltitudeCopter}.svg',
    { query: '?url', import: 'default', eager: true },
  ),
};

export const sectionImageAsset = (image: string): string =>
  Object.entries(SECTION_IMAGES).find(([path]) => path.endsWith(`/${image}`))?.[1] ?? '';

export const SectionHeader = ({ text, image = '' }: { text: string; image?: string }) => {
  const band = useContext(LocalSettingsList);
  const asset = sectionImageAsset(image);
  return (
    <div className={`flex items-center gap-3 ${band ? 'mt-3 w-full bg-surface-container-high px-list py-2.5' : 'px-list pt-5 pb-2'}`}>
      {image.trim() !== '' ? (
        <span
          aria-hidden
          className="block h-6 w-12 shrink-0"
          style={asset ? { backgroundColor: md('onSurface'), maskImage: `url("${asset}")`, maskSize: 'contain', maskRepeat: 'no-repeat', maskPosition: 'center' } : undefined}
        />
      ) : null}
      <Text typography="labelLarge" color={md('onSurfaceVariant')}>
        {text}
      </Text>
    </div>
  );
};

export type SummaryLine = { label: string; value: string; warn: boolean };

export const SummaryLine = (fields: { label: string; value: string; warn?: boolean }): SummaryLine => ({ warn: false, ...fields });

export const SetupRow = ({
  title,
  status = '',
  state = SetupState.Neutral,
  onClick = null,
  summary = [],
  icon = null,
  subtitle = '',
  selected = false,
}: {
  title: string;
  status?: string;
  state?: SetupState;
  onClick?: (() => void) | null;
  summary?: readonly SummaryLine[];
  icon?: DrawableName | null;
  subtitle?: string;
  selected?: boolean;
}) => {
  const stateColor = useSetupStateColor(state);
  const glance = summaryGlanceParts(summary);
  const attention = state === SetupState.NeedsAttention;
  const body = (
    <>
      {icon !== null ? (
        <span className="grid size-10 shrink-0 place-items-center rounded-full" style={{ background: attention ? aircast('warningContainer') : md('secondaryContainer') }}>
          <Drawable name={icon} size={24} style={{ color: attention ? aircast('warning') : md('onSecondaryContainer') }} />
        </span>
      ) : null}
      <span className="flex min-w-0 flex-1 flex-col">
        <Text typography="bodyLarge" color={state === SetupState.Unavailable ? md('onSurfaceVariant') : md('onSurface')}>
          {title}
        </Text>
        {subtitle.trim() !== '' ? (
          <Text typography="bodyMedium" color={md('onSurfaceVariant')} maxLines={1} ellipsis>
            {subtitle}
          </Text>
        ) : null}
        {glance.length > 0 ? (
          <Text typography="bodyMedium" color={md('onSurfaceVariant')} maxLines={1} ellipsis>
            {glanceText(glance, md('error'))}
          </Text>
        ) : null}
      </span>
      {status.trim() !== '' ? (
        <Text typography="labelMedium" color={stateColor}>
          {status}
        </Text>
      ) : null}
      {onClick !== null ? <MaterialIcon name="keyboard_arrow_right" filled style={{ color: md('onSurfaceVariant') }} /> : null}
    </>
  );
  const look = 'flex min-h-[72px] w-full items-center gap-4 px-list py-3 text-left';
  const background = selected ? { background: md('surfaceContainerHighest') } : undefined;
  return onClick !== null ? (
    <button type="button" onClick={onClick} className={`${look} hover:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)]`} style={background}>
      {body}
    </button>
  ) : (
    <div className={look} style={background}>
      {body}
    </div>
  );
};

export const LocalTwoPane = createContext(false);

export class PageHeading {
  constructor(
    readonly title: string,
    readonly back: () => void,
  ) {}
}

export const LocalPageHeading = createContext<MutableState<PageHeading | null> | null>(null);

export const OverridePageHeading = ({ title, onBack }: { title: string; onBack: () => void }) => {
  const host = useContext(LocalPageHeading);
  const back = useUpdatedRef(onBack);
  useEffect(() => {
    if (host) host.value = new PageHeading(title, () => back.current());
    return () => {
      if (host) host.value = null;
    };
  }, [host, title]);
  return null;
};

export const AdvancedToggle = ({ open, className, label = 'Advanced', onToggle }: { open: boolean; className?: string; label?: string; onToggle: () => void }) => (
  <TextButton onClick={onToggle} className={`mx-2 my-2 ${className ?? ''}`}>
    {open ? `Hide ${replaceFirstCharLower(label)}` : label}
    <MaterialIcon name={open ? 'keyboard_arrow_up' : 'keyboard_arrow_down'} filled />
  </TextButton>
);

export const PageTopBar = ({ title, backLabel, onBack }: { title: string; backLabel: string; onBack: () => void }) => {
  const twoPane = useContext(LocalTwoPane);
  return (
    <div className="flex min-h-16 w-full items-center gap-1 px-1">
      {twoPane ? (
        <span className="block w-3 shrink-0" />
      ) : (
        <IconButton onClick={onBack} label={backLabel}>
          <Drawable name="ic_arrow_back" />
        </IconButton>
      )}
      <Text typography="titleLarge" maxLines={1}>
        {title}
      </Text>
    </div>
  );
};

export const EmptyState = ({ icon, title, text, className }: { icon: DrawableName; title: string; text: string; className?: string }) => (
  <div className={`flex w-full flex-col items-center gap-4 px-8 py-12 ${className ?? ''}`}>
    <span className="grid size-24 place-items-center rounded-full bg-secondary-container">
      <Drawable name={icon} size={48} style={{ color: md('onSecondaryContainer') }} />
    </span>
    <Text typography="titleLarge" textAlign="center">
      {sentenceCase(title)}
    </Text>
    {text.trim() !== '' ? (
      <Text typography="bodyMedium" color={md('onSurfaceVariant')} textAlign="center">
        {text}
      </Text>
    ) : null}
  </div>
);

const UNREAD_VALUES: ReadonlySet<string> = new Set(['Unknown', '--', '']);
const GLANCE_SEPARATOR = ' · ';

const firstWord = (value: string): string => (value.includes(' ') ? value.slice(0, value.indexOf(' ')) : '');

const distinctBy = <T,>(items: readonly T[], key: (item: T) => string): T[] =>
  items.filter((item, index) => items.findIndex((other) => key(other) === key(item)) === index);

export const summaryGlanceParts = (summary: readonly SummaryLine[]): SummaryLine[] => {
  const read = summary
    .map((it) => ({ ...it, label: it.label.replace(/[: ]+$/, ''), value: it.value.trim().replace(/(\S)\(/g, '$1 (') }))
    .filter((it) => !UNREAD_VALUES.has(it.value));
  const prefixes = [...new Set(read.map((it) => firstWord(it.value)))];
  const only = prefixes.length === 1 ? (prefixes[0] ?? '') : '';
  const shared = only !== '' && read.length > 1 ? only : null;
  const parts = shared !== null
    ? read.map((it) => ({ ...it, value: `${it.label} ${(it.value.startsWith(shared) ? it.value.slice(shared.length) : it.value).trim()}` }))
    : distinctBy(read, (it) => it.value);
  return parts.map((it) => ({ ...it, value: sentenceCase(it.value) }));
};

export const summaryGlance = (summary: readonly SummaryLine[]): string | null => {
  const parts = summaryGlanceParts(summary);
  return parts.length > 0 ? parts.map((it) => it.value).join(GLANCE_SEPARATOR) : null;
};

const glanceText = (parts: readonly SummaryLine[], warn: string): ReactNode =>
  parts.map((it, index) => (
    <Fragment key={index}>
      {index > 0 ? GLANCE_SEPARATOR : null}
      <span style={it.warn ? { color: warn } : undefined}>{it.value}</span>
    </Fragment>
  ));

export const FootNote = ({ text }: { text: string }) => (
  <div className="flex w-full flex-col gap-1 px-list py-6">
    <Text typography="bodySmall" color={md('onSurfaceVariant')}>
      {text}
    </Text>
  </div>
);

export type ImeAction = 'Default' | 'Done' | 'Go' | 'Next' | 'Search' | 'Send';

export type KeyboardOptions = { imeAction?: ImeAction; keyboardType?: KeyboardType };

export type KeyboardActions = { onDone?: () => void; onGo?: () => void; onNext?: () => void; onSearch?: () => void; onSend?: () => void };

const ENTER_KEY_HINT: Record<ImeAction, 'enter' | 'done' | 'go' | 'next' | 'search' | 'send'> = { Default: 'enter', Done: 'done', Go: 'go', Next: 'next', Search: 'search', Send: 'send' };

const imeHandler = (action: ImeAction, actions: KeyboardActions): (() => void) | undefined =>
  ({ Default: actions.onDone, Done: actions.onDone, Go: actions.onGo, Next: actions.onNext, Search: actions.onSearch, Send: actions.onSend })[action];

const INPUT_MODE: Record<KeyboardType, 'text' | 'numeric' | 'decimal' | 'url' | 'email' | 'tel'> = { text: 'text', number: 'numeric', decimal: 'decimal', password: 'text', uri: 'url', email: 'email', phone: 'tel' };

export const SearchPill = ({
  value,
  onValueChange,
  placeholder,
  className,
  keyboardOptions = {},
  keyboardActions = {},
}: {
  value: string;
  onValueChange: (value: string) => void;
  placeholder: string;
  className?: string;
  keyboardOptions?: KeyboardOptions;
  keyboardActions?: KeyboardActions;
}) => {
  const action = keyboardOptions.imeAction ?? 'Default';
  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    const handler = imeHandler(action, keyboardActions);
    if (event.key !== 'Enter' || !handler) return;
    event.preventDefault();
    handler();
  };
  return (
    <div className={`w-full px-4 py-2 ${className ?? ''}`}>
      <label className="flex min-h-14 w-full items-center gap-3 rounded-full bg-surface-container-high px-4">
        <Drawable name="ic_search" style={{ color: md('onSurfaceVariant') }} />
        <input
          className="min-w-0 flex-1 bg-transparent text-body-large text-on-surface outline-none placeholder:text-on-surface-variant"
          value={value}
          placeholder={placeholder}
          aria-label={placeholder}
          type={keyboardOptions.keyboardType === 'password' ? 'password' : 'text'}
          inputMode={INPUT_MODE[keyboardOptions.keyboardType ?? 'text']}
          enterKeyHint={ENTER_KEY_HINT[action]}
          onChange={(event) => onValueChange(event.target.value)}
          onKeyDown={onKeyDown}
        />
      </label>
    </div>
  );
};

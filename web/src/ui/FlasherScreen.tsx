import { type ReactNode, useEffect, useRef, useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { type Json, has, jsonObjects, jsonStrings, optBoolean, optDouble, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { keepScreenOn } from '@/app/VehicleLinkService';
import { useBackHandler, useLaunchedEffect } from '@/kotlin/Compose';
import { delay, replaceFirstCharUpper } from '@/kotlin/Std';
import { AssistChip, Button, Checkbox, FilterChip, HorizontalDivider, type KeyboardType, LinearProgressIndicator, ListItem, OutlinedButton, OutlinedTextField, RadioButton, Text, TextButton } from '@/m3';
import { aircast, md } from '@/theme/Colors';
import { BlocksNavigation } from '@/ui/AppNavigation';
import { phoneIsOnMeteredNetwork, useRememberPhoneWifi } from '@/ui/PhoneNetwork';
import { OverridePageHeading } from '@/ui/SetupRow';

export const FLASHER_PAGE = 'Aircast card';
export const FLASHER_VIEW = 'view.flasher';
export const FLASHER_OPEN = 'flasher.open';
export const FLASHER_CHANNEL = 'flasher.channel';
export const FLASHER_SELECT = 'flasher.select';
export const FLASHER_DOWNLOAD = 'flasher.download';
export const FLASHER_FORM = 'flasher.form';
export const FLASHER_DISKS = 'flasher.disks';
export const FLASHER_CHOOSE_DISK = 'flasher.chooseDisk';
export const FLASHER_START = 'flasher.start';
export const FLASHER_CANCEL = 'flasher.cancel';
export const FLASHER_AGAIN = 'flasher.again';
export const FLASHER_POLL_MS = 500;
export const FLASHER_DISK_POLL_MS = 2_000;
export const FLASHER_PASSWORD_MODE = 'password';
export const FLASHER_FINISHED: ReadonlySet<string> = new Set(['done', 'failed', 'cancelled']);
export const FLASHER_SETUP_FIELDS: ReadonlySet<string> = new Set(['hostname', 'ssid', 'wifiPassword', 'devicePassword', 'authorizedKey', 'controlServer']);
export const FLASHER_MORE_FIELDS: ReadonlySet<string> = new Set(['authorizedKey', 'controlServer']);
export const FLASHER_FIELD_LABELS: ReadonlyMap<string, string> = new Map([
  ['hostname', 'Drone name'],
  ['ssid', 'WiFi network'],
  ['wifiPassword', 'WiFi password'],
  ['devicePassword', 'Password for pi'],
  ['authorizedKey', 'SSH public key'],
  ['controlServer', 'Headscale server'],
]);
export const FLASHER_MORE_ROWS: ReadonlySet<string> = new Set(['Image', 'Remote access']);

export type FlasherStep = { readonly name: 'Card' | 'Setup' | 'Write'; readonly ordinal: number; readonly title: string };

export const FlasherStep = {
  Card: { name: 'Card', ordinal: 0, title: 'Card' },
  Setup: { name: 'Setup', ordinal: 1, title: 'Setup' },
  Write: { name: 'Write', ordinal: 2, title: 'Write' },
} as const satisfies Record<string, FlasherStep>;

export const FlasherStepEntries: readonly FlasherStep[] = [FlasherStep.Card, FlasherStep.Setup, FlasherStep.Write];

const stepAt = (ordinal: number): FlasherStep => FlasherStepEntries[ordinal] ?? FlasherStep.Card;

export type FlasherRelease = { version: string; label: string; published: string; sizeText: string; recommended: boolean };

export const FlasherRelease = (fields: { version: string; label: string; published: string; sizeText: string; recommended?: boolean }): FlasherRelease => ({ recommended: false, ...fields });

export type FlasherDisk = { id: string; name: string };

export type FlasherSummaryRow = { label: string; value: string; warn: boolean };

export type FlasherForm = {
  hostname: string;
  ssid: string;
  wifiPassword: string;
  noWifi: boolean;
  country: string;
  sshMode: string;
  authorizedKey: string;
  devicePassword: string;
  authKey: string;
  controlServer: string;
};

export const FlasherForm = (fields: Partial<FlasherForm> = {}): FlasherForm => ({
  hostname: '',
  ssid: '',
  wifiPassword: '',
  noWifi: false,
  country: '',
  sshMode: 'key-only',
  authorizedKey: '',
  devicePassword: '',
  authKey: '',
  controlServer: '',
  ...fields,
});

export type FlasherState = {
  available: boolean;
  channel: string;
  channels: readonly string[];
  releasesState: string;
  releasesError: string;
  releases: readonly FlasherRelease[];
  selected: string;
  form: FlasherForm;
  problems: ReadonlyMap<string, string>;
  keyIdentity: string;
  summary: readonly FlasherSummaryRow[];
  disks: readonly FlasherDisk[];
  cardState: string;
  cardId: string;
  cardText: string;
  downloadState: string;
  downloadText: string;
  phase: string;
  busy: boolean;
  percent: number;
  speedText: string;
  jobError: string;
  hostname: string;
  canStart: boolean;
  blocked: string;
  firstProblemField: string;
  firstProblem: string;
  remainingText: string;
};

const notBlank = (text: string): boolean => text.trim() !== '';

export const flasherForm = (json: Json | null): FlasherForm =>
  json === null
    ? FlasherForm()
    : FlasherForm({
        hostname: optText(json, 'hostname'),
        ssid: optText(json, 'ssid'),
        wifiPassword: optText(json, 'wifiPassword'),
        noWifi: optBoolean(json, 'noWifi'),
        country: optText(json, 'country'),
        sshMode: notBlank(optText(json, 'sshMode')) ? optText(json, 'sshMode') : 'key-only',
        authorizedKey: optText(json, 'authorizedKey'),
        devicePassword: optText(json, 'devicePassword'),
        authKey: optText(json, 'authKey'),
        controlServer: optText(json, 'controlServer'),
      });

export const flasherCardText = (card: Json | null): string => {
  switch (card === null ? null : optText(card, 'state')) {
    case 'opening':
      return 'Checking the card reader…';
    case 'waiting':
      return 'Tap OK on the prompt to let Aircast use the card reader';
    case 'ready':
      return [notBlank(optText(card, 'label')) ? optText(card, 'label') : 'Card reader', optText(card, 'capacityText')].filter(notBlank).join(' · ');
    case 'failed':
      return optText(card, 'error');
    default:
      return '';
  }
};

export const flasherDownloadText = (download: Json | null): string => {
  switch (download === null ? null : optText(download, 'state')) {
    case 'running':
      return `Downloading Aircast OS… ${Math.trunc(optDouble(download, 'percent', 0))}%`;
    case 'ready':
      return 'Aircast OS is downloaded';
    case 'failed':
      return `Download failed: ${optText(download, 'error')}`;
    default:
      return '';
  }
};

export const flasherState = (json: Json | null): FlasherState | null => {
  if (json === null || !has(json, 'job')) return null;
  const releases = optJSONObject(json, 'releases');
  const identity = optJSONObject(json, 'keyIdentity');
  const job = optJSONObject(json, 'job');
  const problems = optJSONObject(json, 'problems');
  const card = optJSONObject(json, 'card');
  const download = optJSONObject(json, 'download');
  const firstProblem = optJSONObject(json, 'firstProblem');
  return {
    available: optBoolean(json, 'available'),
    channel: optText(json, 'channel'),
    channels: jsonStrings(optJSONArray(json, 'channels')),
    releasesState: optText(releases, 'state'),
    releasesError: optText(releases, 'error'),
    releases: jsonObjects(optJSONArray(releases, 'items')).map((it) =>
      FlasherRelease({ version: optText(it, 'version'), label: optText(it, 'label'), published: optText(it, 'published'), sizeText: optText(it, 'sizeText'), recommended: optBoolean(it, 'recommended') }),
    ),
    selected: optText(json, 'selected'),
    form: flasherForm(optJSONObject(json, 'form')),
    problems: new Map(Object.keys(problems ?? {}).map((key) => [key, optText(problems, key)])),
    keyIdentity: identity === null ? '' : [optText(identity, 'algorithm'), optText(identity, 'fingerprint'), optText(identity, 'comment')].filter(notBlank).join(' '),
    summary: jsonObjects(optJSONArray(json, 'summary')).map((it) => ({ label: optText(it, 'label'), value: optText(it, 'value'), warn: optBoolean(it, 'warn') })),
    disks: jsonObjects(optJSONArray(json, 'disks')).map((it) => ({ id: optText(it, 'id'), name: optText(it, 'name') })),
    cardState: optText(card, 'state'),
    cardId: optText(card, 'id'),
    cardText: flasherCardText(card),
    downloadState: optText(download, 'state'),
    downloadText: flasherDownloadText(download),
    phase: optText(job, 'phase'),
    busy: optBoolean(job, 'busy'),
    percent: optDouble(job, 'percent', 0),
    speedText: optText(job, 'speedText'),
    jobError: optText(job, 'error'),
    hostname: optText(job, 'hostname'),
    canStart: optBoolean(json, 'canStart'),
    blocked: optText(json, 'blocked'),
    firstProblemField: optText(firstProblem, 'field'),
    firstProblem: optText(firstProblem, 'message'),
    remainingText: optText(job, 'remainingText'),
  };
};

const PHASE_LABELS: Readonly<Record<string, string>> = {
  downloading: 'Downloading Aircast OS',
  preparing: 'Preparing the card',
  writing: 'Writing the card',
  verifying: 'Checking what was written',
  customizing: 'Setting up WiFi and access',
  done: 'Card ready',
  failed: 'Writing failed',
  cancelled: 'Cancelled',
};

export const flasherPhaseLabel = (phase: string): string => PHASE_LABELS[phase] ?? '';

export const flasherStepFor = (state: FlasherState | null): FlasherStep | null => (state !== null && (state.busy || FLASHER_FINISHED.has(state.phase)) ? FlasherStep.Write : null);

export const readerToOpen = (state: FlasherState): string | null => {
  const only = state.disks.length === 1 ? (state.disks[0]?.id ?? null) : null;
  return only !== null && state.cardState === 'none' && !state.busy && !FLASHER_FINISHED.has(state.phase) ? only : null;
};

export const visibleProblems = (problems: ReadonlyMap<string, string>, touched: ReadonlySet<string>, showAll: boolean): ReadonlyMap<string, string> =>
  new Map([...problems].filter(([key]) => showAll || touched.has(key)));

export const setupComplete = (problems: ReadonlyMap<string, string>): boolean => [...problems.keys()].every((it) => !FLASHER_SETUP_FIELDS.has(it));

export const defaultSshMode = (form: FlasherForm): string | null => (form.sshMode === 'key-only' && !notBlank(form.authorizedKey) ? FLASHER_PASSWORD_MODE : null);

export const meteredNote = (release: FlasherRelease | null, downloadState: string, metered: boolean): string | null =>
  release !== null && metered && downloadState !== 'ready' && downloadState !== 'running' ? `Aircast OS is a ${release.sizeText} download and this phone is on mobile data.` : null;

export const FLASHER_SSH_MODES: readonly (readonly [string, string])[] = [
  ['password', 'Password'],
  ['key-only', 'SSH key'],
  ['disabled', 'Off'],
];

export const problemNote = (state: FlasherState): string => {
  if (!notBlank(state.firstProblem)) return '';
  const label = FLASHER_FIELD_LABELS.get(state.firstProblemField);
  return label === undefined ? state.firstProblem : `${label}: ${state.firstProblem}`;
};

export const problemInMoreOptions = (state: FlasherState): boolean => FLASHER_MORE_FIELDS.has(state.firstProblemField);

export const previousStep = (step: FlasherStep, busy: boolean, phase: string): FlasherStep | null =>
  step !== FlasherStep.Card && !busy && !FLASHER_FINISHED.has(phase) ? stepAt(step.ordinal - 1) : null;

export const showsChannels = (debug: boolean, channel: string): boolean => debug || channel !== 'stable';

export const keepsScreenOn = (state: FlasherState | null): boolean => state !== null && (state.busy || state.downloadState === 'running');

export const nextSteps = (ssid: string, hostname: string, noWifi: boolean): string[] => [
  'Unplug the reader, put the card in the drone and power it on.',
  noWifi ? 'Give it about 3 minutes for its first boot on Ethernet or cellular.' : `Give it about 3 minutes for its first boot. It joins ${ssid} by itself.`,
  notBlank(hostname) ? `Open http://${hostname}.local on a phone or computer on the same network to see the drone and its links.` : "Find it on your network under the image's default name.",
];

const patch = (key: string, value: unknown): void => void Qgc.refusalOf(FLASHER_FORM, { [key]: value });

const localeCountry = (): string => {
  try {
    return new Intl.Locale(navigator.language).region ?? '';
  } catch {
    return '';
  }
};

type Edit = (next: FlasherForm, key: string, value: unknown) => void;

export const FlasherScreen = ({ className }: { className?: string }) => {
  const phoneWifi = useRememberPhoneWifi();
  const phoneSsid = phoneWifi.ssid;
  const [more, setMore] = useState(false);
  const [state, setState] = useState<FlasherState | null>(null);
  const formRef = useRef<FlasherForm | null>(null);
  const [form, setFormState] = useState<FlasherForm | null>(null);
  const [step, setStep] = useState<FlasherStep>(FlasherStep.Card);
  const [touched, setTouched] = useState<ReadonlySet<string>>(new Set());
  const [showAllProblems, setShowAllProblems] = useState(false);
  const [refusal, setRefusal] = useState('');
  const [metered, setMetered] = useState(false);
  const setForm = (next: FlasherForm | null) => {
    formRef.current = next;
    setFormState(next);
  };

  useLaunchedEffect([], async (scope) => {
    await Qgc.invoke(FLASHER_OPEN);
    const poll = async (): Promise<void> => {
      if (!scope.isActive()) return;
      const read = flasherState(await Qgc.get(FLASHER_VIEW));
      if (!scope.isActive()) return;
      setState(read);
      setMetered(phoneIsOnMeteredNetwork());
      const shownStep = flasherStepFor(read);
      if (shownStep !== null) setStep(shownStep);
      if (formRef.current === null && read !== null) {
        const country = notBlank(read.form.country) ? read.form.country : localeCountry();
        const sshMode = defaultSshMode(read.form) ?? read.form.sshMode;
        if (country !== read.form.country) patch('country', country);
        if (sshMode !== read.form.sshMode) patch('sshMode', sshMode);
        setForm({ ...read.form, country, sshMode });
      }
      const reader = read === null ? null : readerToOpen(read);
      if (reader !== null) await Qgc.invoke(FLASHER_CHOOSE_DISK, reader);
      await delay(FLASHER_POLL_MS);
      return poll();
    };
    await poll();
  });

  useLaunchedEffect([], async (scope) => {
    const poll = async (): Promise<void> => {
      await delay(FLASHER_DISK_POLL_MS);
      if (!scope.isActive()) return;
      await Qgc.invoke(FLASHER_DISKS);
      return poll();
    };
    await poll();
  });

  useEffect(() => {
    const current = formRef.current;
    if (current === null || phoneSsid === null) return;
    if (!notBlank(current.ssid) && !current.noWifi) {
      setForm({ ...current, ssid: phoneSsid });
      patch('ssid', phoneSsid);
    }
  }, [phoneSsid, form === null]);

  useEffect(() => {
    if (formRef.current !== null) patch('securedSsid', phoneWifi.secured === true ? (phoneWifi.ssid ?? '') : '');
  }, [phoneWifi.ssid, phoneWifi.secured, form === null]);

  const awake = keepsScreenOn(state);
  useEffect(() => (awake ? keepScreenOn() : undefined), [awake]);

  const busyRef = useRef(false);
  busyRef.current = state?.busy === true;
  useEffect(
    () => () => {
      if (busyRef.current) void Qgc.invoke(FLASHER_CANCEL);
    },
    [],
  );

  const back = previousStep(step, state?.busy === true, state?.phase ?? '');
  useBackHandler(back !== null, () => {
    if (back !== null) setStep(back);
  });

  const command = (path: string, args: readonly unknown[]) =>
    void (async () => {
      setRefusal((await Qgc.refusalOf(path, ...args)) ?? '');
    })();
  const edit: Edit = (next, key, value) => {
    setForm(next);
    setTouched((previous) => new Set([...previous, key]));
    patch(key, value);
  };
  const startDownload = () => command(FLASHER_DOWNLOAD, []);

  const body = (current: FlasherState, fields: FlasherForm) => {
    const release = current.releases.find((it) => it.version === current.selected) ?? null;
    const note = meteredNote(release, current.downloadState, metered);
    const again = () => {
      const finished = current.phase;
      void (async () => {
        await Qgc.refusalOf(FLASHER_AGAIN);
        const next = flasherState(await Qgc.get(FLASHER_VIEW));
        if (next !== null) {
          setForm(next.form);
          setState(next);
        }
        setTouched(new Set());
        setShowAllProblems(false);
        setStep(finished === 'done' ? FlasherStep.Card : FlasherStep.Write);
      })();
    };
    const proceed = () => {
      setRefusal('');
      if (step === FlasherStep.Card) {
        setStep(FlasherStep.Setup);
        if (!metered && ['idle', 'failed', ''].includes(current.downloadState)) startDownload();
        return;
      }
      if (step === FlasherStep.Setup) {
        setShowAllProblems(true);
        if (problemInMoreOptions(current)) setMore(true);
        if (setupComplete(current.problems)) setStep(FlasherStep.Write);
        return;
      }
      command(FLASHER_START, []);
    };
    return (
      <>
        <div className="flex min-h-0 w-full flex-1 flex-col overflow-auto py-2">
          {step === FlasherStep.Card ? <CardStep state={current} onChoose={(it) => command(FLASHER_CHOOSE_DISK, [it])} /> : null}
          {step === FlasherStep.Setup ? (
            <SetupStep
              state={current}
              form={fields}
              phoneSsid={phoneSsid}
              more={more}
              onMore={() => setMore(!more)}
              problems={visibleProblems(current.problems, touched, showAllProblems)}
              meteredNote={note}
              edit={edit}
              onSelect={(it) => command(FLASHER_SELECT, [it])}
              onChannel={(it) => command(FLASHER_CHANNEL, [it])}
              onDownload={startDownload}
            />
          ) : null}
          {step === FlasherStep.Write ? (
            <WriteStep
              state={current}
              form={fields}
              meteredNote={note}
              onChange={(row) => {
                if (FLASHER_MORE_ROWS.has(row)) setMore(true);
                setStep(row === 'Card' ? FlasherStep.Card : FlasherStep.Setup);
              }}
            />
          ) : null}
        </div>
        <HorizontalDivider />
        <StepActions
          step={step}
          state={current}
          refusal={refusal}
          setupNote={showAllProblems ? problemNote(current) : ''}
          onBack={() => {
            setRefusal('');
            setStep(stepAt(step.ordinal - 1));
          }}
          onContinue={proceed}
          onCancel={() => command(FLASHER_CANCEL, [])}
          onAgain={again}
        />
      </>
    );
  };

  return (
    <div className={`flex h-full w-full flex-col ${className ?? ''}`}>
      <BlocksNavigation blocking={state?.busy === true} reason="Wait for the card to finish writing or cancel it first" />
      {back !== null ? <OverridePageHeading title={FLASHER_PAGE} onBack={() => setStep(back)} /> : null}
      <StepHeader step={step} onStep={(chosen) => back !== null && chosen.ordinal < step.ordinal && setStep(chosen)} />
      <HorizontalDivider />
      {state === null || form === null ? (
        <Text className="block p-4">Loading…</Text>
      ) : !state.available ? (
        <Text color={aircast('warning')} className="block p-4">
          This device cannot write cards.
        </Text>
      ) : (
        body(state, form)
      )}
    </div>
  );
};

const StepHeader = ({ step, onStep }: { step: FlasherStep; onStep: (step: FlasherStep) => void }) => (
  <div className="flex w-full justify-between px-2 py-1">
    {FlasherStepEntries.map((each) => (
      <button
        key={each.name}
        type="button"
        disabled={each.ordinal >= step.ordinal}
        onClick={() => onStep(each)}
        className="whitespace-pre px-2 py-2 text-left disabled:cursor-default"
      >
        <Text typography="labelLarge" fontWeight={each === step ? 700 : 400} color={each.ordinal <= step.ordinal ? md('primary') : md('onSurfaceVariant')}>
          {`${each.ordinal < step.ordinal ? '✓' : `${each.ordinal + 1}`}  ${each.title}`}
        </Text>
      </button>
    ))}
  </div>
);

const StepActions = ({
  step,
  state,
  refusal,
  setupNote,
  onBack,
  onContinue,
  onCancel,
  onAgain,
}: {
  step: FlasherStep;
  state: FlasherState;
  refusal: string;
  setupNote: string;
  onBack: () => void;
  onContinue: () => void;
  onCancel: () => void;
  onAgain: () => void;
}) => {
  const note = notBlank(refusal)
    ? refusal
    : step === FlasherStep.Setup && notBlank(setupNote)
      ? setupNote
      : step === FlasherStep.Write && !state.busy && !FLASHER_FINISHED.has(state.phase)
        ? state.blocked
        : '';
  const actions = () => {
    if (state.busy) {
      return (
        <OutlinedButton onClick={onCancel} className="flex-1">
          Cancel
        </OutlinedButton>
      );
    }
    if (FLASHER_FINISHED.has(state.phase)) {
      return (
        <Button onClick={onAgain} className="flex-1">
          {state.phase === 'done' ? 'Flash another card' : 'Try again'}
        </Button>
      );
    }
    return (
      <>
        {step !== FlasherStep.Card ? (
          <OutlinedButton onClick={onBack} className="flex-1">
            Back
          </OutlinedButton>
        ) : null}
        <Button onClick={onContinue} enabled={step === FlasherStep.Write ? state.canStart : true} className="flex-1">
          {step === FlasherStep.Write ? 'Write card' : 'Continue'}
        </Button>
      </>
    );
  };
  return (
    <div className="flex w-full flex-col gap-2 p-4">
      {notBlank(note) ? (
        <Text typography="bodyMedium" color={aircast('warning')}>
          {note}
        </Text>
      ) : null}
      <div className="flex w-full gap-3">{actions()}</div>
    </div>
  );
};

const Heading = ({ title, body }: { title: string; body: string }) => (
  <div className="flex w-full flex-col gap-1 px-4 py-2">
    <Text typography="titleLarge">{title}</Text>
    <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
      {body}
    </Text>
  </div>
);

const Selectable = ({ onClick, children }: { onClick: () => void; children: ReactNode }) => (
  <div className="w-full cursor-pointer hover:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)]" onClick={onClick}>
    {children}
  </div>
);

const CardStep = ({ state, onChoose }: { state: FlasherState; onChoose: (id: string) => void }) => (
  <>
    <Heading title="Insert the card" body="Put the drone's SD card in a USB card reader and plug it into this phone. Use an OTG adapter if the reader has a full-size USB plug." />
    {state.disks.length === 0 ? <ListItem headlineContent="Waiting for a card reader…" supportingContent="You can continue and plug it in before writing" /> : null}
    {state.disks.map((disk) => {
      const chosen = disk.id === state.cardId;
      const detail = chosen && notBlank(state.cardText) ? state.cardText : null;
      return (
        <Selectable key={disk.id} onClick={() => onChoose(disk.id)}>
          <ListItem
            headlineContent={disk.name}
            supportingContent={detail !== null ? <span style={{ color: state.cardState === 'failed' ? aircast('warning') : md('onSurfaceVariant') }}>{detail}</span> : undefined}
            leadingContent={<RadioButton selected={chosen} onClick={null} label={disk.name} />}
            trailingContent={chosen && state.cardState === 'ready' ? <Text color={md('primary')}>✓</Text> : undefined}
          />
        </Selectable>
      );
    })}
    {state.cardState === 'failed' && notBlank(state.cardId) ? (
      <div className="px-2">
        <TextButton onClick={() => onChoose(state.cardId)}>Try the reader again</TextButton>
      </div>
    ) : null}
  </>
);

const Field = ({
  label,
  value,
  problem,
  help = null,
  secret = false,
  singleLine = true,
  keyboard = 'text',
  onChange,
}: {
  label: string;
  value: string;
  problem: string | null;
  help?: string | null;
  secret?: boolean;
  singleLine?: boolean;
  keyboard?: KeyboardType;
  onChange: (value: string) => void;
}) => (
  <OutlinedTextField
    value={value}
    onValueChange={onChange}
    label={label}
    singleLine={singleLine}
    isError={problem !== null}
    supportingText={problem ?? help ?? undefined}
    keyboardType={secret ? 'password' : keyboard}
    className="w-full px-4 py-1"
  />
);

const SetupStep = ({
  state,
  form,
  phoneSsid,
  more,
  onMore,
  problems,
  meteredNote,
  edit,
  onSelect,
  onChannel,
  onDownload,
}: {
  state: FlasherState;
  form: FlasherForm;
  phoneSsid: string | null;
  more: boolean;
  onMore: () => void;
  problems: ReadonlyMap<string, string>;
  meteredNote: string | null;
  edit: Edit;
  onSelect: (version: string) => void;
  onChannel: (channel: string) => void;
  onDownload: () => void;
}) => (
  <>
    <Heading title="Set up the drone" body="These are applied on the drone's first boot, so it joins your network by itself." />
    <Field
      label="Drone name"
      value={form.hostname}
      problem={problems.get('hostname') ?? null}
      help={notBlank(form.hostname) ? `It answers at ${form.hostname}.local` : null}
      onChange={(it) => edit({ ...form, hostname: it }, 'hostname', it)}
    />
    {!form.noWifi ? (
      <>
        <Field
          label="WiFi network"
          value={form.ssid}
          problem={problems.get('ssid') ?? null}
          help={phoneSsid !== null && form.ssid === phoneSsid ? "This phone's current network" : null}
          onChange={(it) => edit({ ...form, ssid: it }, 'ssid', it)}
        />
        {phoneSsid !== null && phoneSsid !== form.ssid ? (
          <div className="px-4">
            <AssistChip onClick={() => edit({ ...form, ssid: phoneSsid }, 'ssid', phoneSsid)} label={`Use ${phoneSsid}`} />
          </div>
        ) : null}
        <Field label="WiFi password" value={form.wifiPassword} problem={problems.get('wifiPassword') ?? null} secret onChange={(it) => edit({ ...form, wifiPassword: it }, 'wifiPassword', it)} />
      </>
    ) : null}
    <Selectable onClick={() => edit({ ...form, noWifi: !form.noWifi }, 'noWifi', !form.noWifi)}>
      <ListItem headlineContent="No WiFi" supportingContent="The drone goes online over Ethernet or cellular" trailingContent={<Checkbox checked={form.noWifi} onCheckedChange={null} label="No WiFi" />} />
    </Selectable>
    {form.sshMode === FLASHER_PASSWORD_MODE ? (
      <Field
        label="Password for pi"
        value={form.devicePassword}
        problem={problems.get('devicePassword') ?? null}
        help="At least 8 characters. You sign in to the drone as pi with it."
        secret
        onChange={(it) => edit({ ...form, devicePassword: it }, 'devicePassword', it)}
      />
    ) : null}
    <DownloadLine state={state} meteredNote={meteredNote} onDownload={onDownload} />
    <div className="px-2">
      <TextButton onClick={onMore}>{more ? 'Fewer options' : 'More options'}</TextButton>
    </div>
    {more ? <MoreOptions state={state} form={form} problems={problems} edit={edit} onSelect={onSelect} onChannel={onChannel} /> : null}
  </>
);

const DownloadLine = ({ state, meteredNote, onDownload }: { state: FlasherState; meteredNote: string | null; onDownload: () => void }) => {
  if (meteredNote !== null) {
    return (
      <div className="flex flex-col items-start gap-1 px-4 py-1">
        <Text typography="bodySmall" color={aircast('warning')}>
          {meteredNote}
        </Text>
        <TextButton onClick={onDownload}>Download now anyway</TextButton>
      </div>
    );
  }
  return notBlank(state.downloadText) ? (
    <Text typography="bodySmall" color={md('onSurfaceVariant')} className="block px-4 py-1">
      {state.downloadText}
    </Text>
  ) : null;
};

const MoreOptions = ({
  state,
  form,
  problems,
  edit,
  onSelect,
  onChannel,
}: {
  state: FlasherState;
  form: FlasherForm;
  problems: ReadonlyMap<string, string>;
  edit: Edit;
  onSelect: (version: string) => void;
  onChannel: (channel: string) => void;
}) => (
  <>
    <SubHeading text="Aircast OS version" />
    {state.releases.map((release) => (
      <Selectable key={release.version} onClick={() => onSelect(release.version)}>
        <ListItem
          headlineContent={release.label}
          supportingContent={[release.published, `${release.sizeText} download`, release.recommended ? 'Recommended' : null].filter((it): it is string => it !== null).join(' · ')}
          leadingContent={<RadioButton selected={release.version === state.selected} onClick={null} label={release.label} />}
        />
      </Selectable>
    ))}
    {state.releasesState === 'failed' ? (
      <Text color={aircast('warning')} className="block px-4">
        {state.releasesError}
      </Text>
    ) : null}
    {showsChannels(import.meta.env.DEV, state.channel) ? (
      <div className="flex w-full flex-wrap gap-2 px-4">
        {state.channels.map((channel) => (
          <FilterChip key={channel} selected={channel === state.channel} onClick={() => onChannel(channel)} label={replaceFirstCharUpper(channel)} />
        ))}
      </div>
    ) : null}
    <SubHeading text="Signing in to the drone" />
    <div className="flex w-full flex-wrap gap-2 px-4">
      {FLASHER_SSH_MODES.map(([mode, label]) => (
        <FilterChip key={mode} selected={form.sshMode === mode} onClick={() => edit({ ...form, sshMode: mode }, 'sshMode', mode)} label={label} />
      ))}
    </div>
    {form.sshMode === 'key-only' ? (
      <>
        <Field label="SSH public key" value={form.authorizedKey} problem={problems.get('authorizedKey') ?? null} singleLine={false} onChange={(it) => edit({ ...form, authorizedKey: it }, 'authorizedKey', it)} />
        {notBlank(state.keyIdentity) ? (
          <Text typography="bodySmall" monospace className="block px-4">
            {state.keyIdentity}
          </Text>
        ) : null}
      </>
    ) : null}
    <SubHeading text="Remote access" />
    <Field label="Tailscale auth key" value={form.authKey} problem={null} help="The drone joins your tailnet on first boot" secret onChange={(it) => edit({ ...form, authKey: it }, 'authKey', it)} />
    {notBlank(form.authKey) ? (
      <Field
        label="Headscale server"
        value={form.controlServer}
        problem={problems.get('controlServer') ?? null}
        help="Leave empty for Tailscale"
        keyboard="uri"
        onChange={(it) => edit({ ...form, controlServer: it }, 'controlServer', it)}
      />
    ) : null}
    <SubHeading text="WiFi region" />
    <Field
      label="Country code"
      value={form.country}
      problem={null}
      help="Sets which WiFi channels the drone may use"
      onChange={(value) => {
        const country = value.toUpperCase().slice(0, 2);
        edit({ ...form, country }, 'country', country);
      }}
    />
    <div className="h-2" />
  </>
);

const SubHeading = ({ text }: { text: string }) => (
  <Text typography="titleSmall" color={md('primary')} className="block px-4 pt-4 pb-1">
    {text}
  </Text>
);

const WriteStep = ({ state, form, meteredNote, onChange }: { state: FlasherState; form: FlasherForm; meteredNote: string | null; onChange: (row: string) => void }) => {
  if (state.busy) {
    return (
      <div className="flex w-full flex-col gap-3 p-4">
        <Text typography="titleLarge">{flasherPhaseLabel(state.phase)}</Text>
        <LinearProgressIndicator progress={state.percent / 100} />
        <Text typography="bodyMedium">{[`${Math.trunc(state.percent)}%`, state.speedText, state.remainingText].filter(notBlank).join(' · ')}</Text>
        <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
          Keep the card reader plugged in until this finishes.
        </Text>
      </div>
    );
  }
  if (state.phase === 'done') {
    return (
      <div className="flex w-full flex-col">
        <Heading title="Card ready" body="The card is written and checked. It's safe to unplug the reader." />
        <SubHeading text="What happens next" />
        {nextSteps(form.ssid.trim(), state.hostname, form.noWifi).map((line, index) => (
          <Text key={index} typography="bodyMedium" className="block px-4 py-1">
            {`${index + 1}. ${line}`}
          </Text>
        ))}
      </div>
    );
  }
  if (state.phase === 'failed') {
    return (
      <div className="flex w-full flex-col">
        <Heading title="Writing failed" body="The card is not usable as it is. Fix the problem below and write it again." />
        <Text typography="bodyMedium" color={aircast('warning')} className="block px-4">
          {state.jobError}
        </Text>
      </div>
    );
  }
  if (state.phase === 'cancelled') return <Heading title="Cancelled" body="The card is incomplete. Write it again before putting it in the drone." />;
  return (
    <>
      <Heading title="Check and write" body="Everything on the card is replaced." />
      {state.summary.map((row) => (
        <div key={row.label} className="flex w-full items-center gap-2 pr-1 pl-4">
          <Text typography="bodyMedium" className="flex-1">
            {row.label}
          </Text>
          <Text typography="bodyMedium" color={row.warn ? aircast('warning') : md('onSurface')}>
            {row.value}
          </Text>
          <TextButton onClick={() => onChange(row.label)}>Change</TextButton>
        </div>
      ))}
      <div className="flex w-full items-center gap-2 pr-1 pl-4">
        <Text typography="bodyMedium" className="flex-1">
          Card
        </Text>
        <Text typography="bodyMedium" color={state.cardState === 'ready' ? md('onSurface') : aircast('warning')}>
          {notBlank(state.cardText) ? state.cardText : 'Not connected'}
        </Text>
        <TextButton onClick={() => onChange('Card')}>Change</TextButton>
      </div>
      {state.downloadState !== 'ready' && notBlank(state.downloadText) ? (
        <Text typography="bodySmall" color={md('onSurfaceVariant')} className="block px-4 py-1">
          {state.downloadText}
        </Text>
      ) : null}
      {meteredNote !== null ? (
        <Text typography="bodySmall" color={aircast('warning')} className="block px-4 py-1">
          {`${meteredNote} It downloads when you write the card.`}
        </Text>
      ) : null}
    </>
  );
};

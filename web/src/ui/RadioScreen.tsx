import { useEffect, useMemo, useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { useKeyedState, useLaunchedEffect } from '@/kotlin/Compose';
import { AlertDialog, Button, Checkbox, FilterChip, HorizontalDivider, OutlinedButton, RadioButton, Surface, Text, TextButton } from '@/m3';
import { sentenceCase } from '@/map/SentenceCase';
import { md } from '@/theme/Colors';
import { StickDiagram } from '@/ui/JoystickScreen';
import { type ParameterRows, readPage } from '@/ui/ParameterForm';
import {
  RADIO_CAL,
  RADIO_PROMPTS,
  RADIO_VIEW,
  type RadioCalibration,
  type RadioPrompt,
  type RadioStick,
  type RadioView,
  THROTTLE_REVERSED_TEXT,
  THROTTLE_REVERSED_TITLE,
  calibrationStep,
  radioCalAction,
  radioView,
} from '@/ui/RadioView';
import { FactRow } from '@/ui/SettingsScreen';
import { FootNote, SectionHeader } from '@/ui/SetupRow';

const RadioNotice = ({ text, className }: { text: string; className?: string }) => (
  <Text typography="bodyLarge" textAlign="center" className={`w-full p-6 ${className ?? ''}`}>
    {text}
  </Text>
);

export const PwmBar = ({ fraction, className }: { fraction: number; className?: string }) => (
  <div className={`h-2 overflow-hidden rounded-[4px] ${className ?? ''}`} style={{ background: md('surfaceVariant') }}>
    <div className="h-full" style={{ width: `${fraction * 100}%`, background: md('primary') }} />
  </div>
);

const AttitudeRow = ({ stick }: { stick: RadioStick }) => (
  <div className="flex w-full flex-col gap-1.5 px-4 py-2">
    <div className="flex items-center">
      <Text typography="bodyLarge" className="flex-1">
        {stick.title}
      </Text>
      <Text typography="labelMedium" color={stick.mapped ? md('onSurfaceVariant') : md('error')}>
        {stickReading(stick)}
      </Text>
    </div>
    {stick.mapped ? <PwmBar fraction={stick.fraction} className="w-full" /> : null}
  </div>
);

const CalibrationStep = ({ cal, onAction }: { cal: RadioCalibration; onAction: (action: string) => void }) => (
  <Surface className="mx-4 my-2 block" color={md('primaryContainer')} shape={12}>
    <div className="flex flex-col p-4">
      <Text typography="titleMedium" className="block w-full whitespace-pre-line pb-4">
        {calibrationStep(cal.statusText)}
      </Text>
      {cal.running ? <StickDiagram positions={cal.stickPositions} single={false} /> : null}
      <div className="flex items-center gap-2">
        <Button onClick={() => onAction('nextButtonClicked')} enabled={cal.nextEnabled}>
          {cal.nextText.trim() === '' ? 'Next' : cal.nextText}
        </Button>
        {cal.skipEnabled ? <OutlinedButton onClick={() => onAction('skipButtonClicked')}>Skip</OutlinedButton> : null}
        <span className="flex-1" />
        {cal.cancelEnabled ? <TextButton onClick={() => onAction('cancelButtonClicked')}>Cancel</TextButton> : null}
      </div>
    </div>
  </Surface>
);

const MODES = [1, 2, 3, 4];

const ModeRow = ({ mode, centeredThrottle, onPick, onCentered }: { mode: number; centeredThrottle: boolean | null; onPick: (mode: number) => void; onCentered: (centered: boolean) => void }) => (
  <div className="flex w-full flex-col px-5 py-1">
    <Text typography="bodySmall" className="block pb-1">
      Transmitter mode — which stick is the throttle
    </Text>
    <div className="flex flex-wrap gap-2">
      {MODES.map((choice) => (
        <FilterChip key={choice} selected={choice === mode} onClick={() => onPick(choice)} label={`Mode ${choice}`} />
      ))}
    </div>
    {centeredThrottle !== null ? (
      <div className="flex items-center gap-3 py-3">
        <Checkbox checked={centeredThrottle} onCheckedChange={onCentered} label="Centered throttle" />
        <Text typography="bodyMedium">Centered throttle</Text>
      </div>
    ) : null}
  </div>
);

const ConfirmDialog = ({ prompt, onDismiss, onConfirm }: { prompt: RadioPrompt; onDismiss: () => void; onConfirm: (choice: number | null) => void }) => {
  const [choice, setChoice] = useKeyedState<number | null>([prompt], () => (prompt.choices.length > 0 ? prompt.choices.length - 1 : null));
  return (
    <AlertDialog
      onDismissRequest={onDismiss}
      title={prompt.title}
      text={
        <div className="flex flex-col">
          <span className="whitespace-pre-line">{prompt.body}</span>
          {prompt.choices.length > 0 ? (
            <div role="radiogroup" className="flex flex-col">
              {prompt.choices.map((label, index) => (
                <div key={label} className="flex w-full cursor-pointer items-center gap-2 py-2" onClick={() => setChoice(index)}>
                  <RadioButton selected={index === choice} onClick={() => setChoice(index)} label={label} />
                  <span>{label}</span>
                </div>
              ))}
            </div>
          ) : null}
        </div>
      }
      confirmButton={
        <TextButton
          onClick={() => {
            onConfirm(choice);
            onDismiss();
          }}
        >
          Ok
        </TextButton>
      }
      dismissButton={<TextButton onClick={onDismiss}>Cancel</TextButton>}
    />
  );
};

const AdditionalSetup = ({ onInvoke }: { onInvoke: (action: string, choice: number | null) => void }) => {
  const [prompt, setPrompt] = useState<RadioPrompt | null>(null);
  const asked = prompt;
  return (
    <>
      {asked !== null ? <ConfirmDialog prompt={asked} onDismiss={() => setPrompt(null)} onConfirm={(choice) => onInvoke(asked.action, choice)} /> : null}
      <div className="flex w-full flex-col px-1 py-1">
        <HorizontalDivider className="mb-2" />
        <div className="flex flex-wrap gap-2">
          {RADIO_PROMPTS.map((entry) => (
            <OutlinedButton key={entry.action} onClick={() => setPrompt(entry)}>
              {entry.title}
            </OutlinedButton>
          ))}
        </div>
      </div>
    </>
  );
};

const CalibrationStart = ({
  view,
  onAction,
  onMode,
  onCentered,
  onInvoke,
}: {
  view: RadioView;
  onAction: (action: string) => void;
  onMode: (mode: number) => void;
  onCentered: (centered: boolean) => void;
  onInvoke: (action: string, choice: number | null) => void;
}) => {
  const [prompting, setPrompting] = useState(false);
  const [refusing, setRefusing] = useState(false);
  const startPrompt = prompting ? view.startPrompt : null;
  const notReady = refusing ? view.notReady : null;
  return (
    <>
      {startPrompt !== null ? (
        <AlertDialog
          onDismissRequest={() => setPrompting(false)}
          title={startPrompt[0]}
          text={<span className="whitespace-pre-line">{startPrompt[1]}</span>}
          confirmButton={
            <TextButton
              onClick={() => {
                setPrompting(false);
                onAction('nextButtonClicked');
              }}
            >
              Ok
            </TextButton>
          }
        />
      ) : null}
      {notReady !== null ? (
        <AlertDialog
          onDismissRequest={() => setRefusing(false)}
          title={notReady[0]}
          text={<span className="whitespace-pre-line">{notReady[1]}</span>}
          confirmButton={<TextButton onClick={() => setRefusing(false)}>Ok</TextButton>}
        />
      ) : null}
      <div className="flex w-full flex-col px-4 py-2">
        <ModeRow mode={view.transmitterMode} centeredThrottle={view.joystickMode ? null : view.centeredThrottle} onPick={onMode} onCentered={onCentered} />
        <div className="flex w-full items-center gap-3 px-1 py-2">
          <Button
            onClick={() => (view.notReady !== null ? setRefusing(true) : view.startPrompt !== null ? setPrompting(true) : onAction('nextButtonClicked'))}
            enabled={view.calibration.nextEnabled}
          >
            {view.calibration.nextText.trim() === '' ? 'Calibrate' : view.calibration.nextText}
          </Button>
          <Text typography="bodySmall" color={md('onSurfaceVariant')}>
            {view.summary}
          </Text>
        </div>
        <AdditionalSetup onInvoke={onInvoke} />
      </div>
    </>
  );
};

export const RADIO_SWITCHES_PAGE = 'Radio Switches';
export const RADIO_ENTER_ACTIONS: readonly string[] = ['start', 'open'];
export const RADIO_LEAVE_ACTIONS: readonly string[] = ['cancelButtonClicked', 'close'];

const invokeInOrder = (actions: readonly string[]): void => void Promise.all(actions.map((it) => Qgc.invoke(radioCalAction(it))));

const invokeAction = (action: string): void => void Qgc.invoke(radioCalAction(action));

export const RadioScreen = ({ className }: { className?: string }) => {
  const json = useQgcPath(RADIO_VIEW);
  const view = useMemo(() => radioView(json), [json]);
  useEffect(() => {
    invokeInOrder(RADIO_ENTER_ACTIONS);
    return () => invokeInOrder(RADIO_LEAVE_ACTIONS);
  }, []);
  const [switchReads, setSwitchReads] = useState(0);
  const [switches, setSwitches] = useState<readonly ParameterRows[]>([]);
  useLaunchedEffect([switchReads, view?.connected ?? null], async (scope) => {
    const read = await readPage(RADIO_SWITCHES_PAGE);
    if (scope.isActive()) setSwitches(read);
  });
  const connected = view !== null && view.connected;
  const reversed = view?.calibration.throttleReversed ?? false;
  const [reversedSeen, setReversedSeen] = useKeyedState([reversed, connected], () => false);

  if (view === null || !view.connected) {
    return <RadioNotice text="Connect a vehicle to check its radio." className={className} />;
  }

  return (
    <>
      {reversed && !reversedSeen ? (
        <AlertDialog
          onDismissRequest={() => setReversedSeen(true)}
          title={THROTTLE_REVERSED_TITLE}
          text={THROTTLE_REVERSED_TEXT}
          confirmButton={<TextButton onClick={() => setReversedSeen(true)}>OK</TextButton>}
        />
      ) : null}
      <div className={`flex h-full w-full flex-col overflow-y-auto ${className ?? ''}`}>
        {view.channelCount === 0 ? <RadioNotice text="No transmitter signal. Turn the transmitter on and check the receiver is bound." /> : null}

        {view.calibration.running ? (
          <CalibrationStep cal={view.calibration} onAction={invokeAction} />
        ) : (
          <CalibrationStart
            view={view}
            onAction={invokeAction}
            onMode={(mode) => void Qgc.set(`${RADIO_CAL}.transmitterMode`, mode)}
            onCentered={(centered) => void Qgc.set(`${RADIO_CAL}.centeredThrottle`, centered)}
            onInvoke={(action, choice) => void (choice === null ? Qgc.invoke(radioCalAction(action)) : Qgc.invoke(radioCalAction(action), choice))}
          />
        )}

        <SectionHeader text="Attitude controls" />
        {view.sticks.map((stick) => (
          <AttitudeRow key={`att${stick.title}`} stick={stick} />
        ))}

        {switches.map((section) => (
          <div key={`switches:${section.title}`} className="flex flex-col">
            <SectionHeader text={sentenceCase(section.title)} />
            {section.facts.map((fact) => (
              <FactRow key={`switch:${fact.path}`} fact={fact} onWrite={() => setSwitchReads((it) => it + 1)} />
            ))}
          </div>
        ))}

        <SectionHeader text="Raw channel monitor" />
        {view.channels.map((channel) => (
          <div key={`ch${channel.label}`} className="flex w-full items-center gap-3 px-5 py-1.5">
            <Text typography="bodySmall" monospace className="w-7 shrink-0">
              {channel.label}
            </Text>
            <PwmBar fraction={channel.fraction} className="flex-1" />
            <Text typography="bodySmall" monospace className="w-12 shrink-0">
              {channel.valueText}
            </Text>
          </div>
        ))}

        <FootNote
          text="Move each stick and switch — every channel you use should move here. Calibration asks you to hold each stick at its extremes in turn and rewrites the mapping, so do it with the propellers off."
        />
      </div>
    </>
  );
};

export const stickReading = (stick: RadioStick): string =>
  !stick.mapped
    ? 'Not mapped'
    : [stick.channel !== null ? `Ch ${stick.channel}` : null, stick.valueText.trim() !== '' ? `${stick.valueText} µs` : null, stick.reversed ? 'reversed' : null]
        .filter((it): it is string => it !== null)
        .join(' · ');

import { type CSSProperties, Fragment, type ReactNode, createContext, useContext, useEffect, useMemo, useState } from 'react';
import { Menu } from '@base-ui/react';
import { type Json, isNull, jsonObjects, optBoolean, optDouble, optJSONArray, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { settingControl, useQgcBool, useQgcPath, useQgcPathHoldingLast } from '@/bridge/QgcState';
import { VehicleCommands } from '@/bridge/VehicleCommands';
import { useKeyedState, useLaunchedEffect, useUpdatedRef } from '@/kotlin/Compose';
import { useValue } from '@/kotlin/State';
import { delay } from '@/kotlin/Std';
import { AlertDialog, Checkbox, DropdownMenu, HorizontalDivider, IconButton, ListItem, Surface, Switch, Text, TextButton, type Typography } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { sentenceCase } from '@/map/SentenceCase';
import { Drawable, MaterialIcon } from '@/res/Icon';
import type { DrawableName } from '@/res/drawables';
import { aircast, argb, md, withAlpha } from '@/theme/Colors';
import { useRememberActiveVehicleId } from './ActiveVehicle';
import { useAdvancedUiShown } from './AdvancedUi';
import { LocalAppNavigation } from './AppNavigation';
import { ConfirmTrack, SENT_SHOWN_MS, SentNotice, sentIsStillShowing, sentUnlessRefused } from './ConfirmTrack';
import { armedStopOffer, emergencyStopAction } from './EmergencyStop';
import { FLIGHT_MODES, type FlightModeOption, flightModesView, modeAck, modeHeading, modeOutcome, pinsAfter, startsSection } from './FlightModes';
import { CHECKLIST, DeckButton, DeckEntry, type DeckIds, FlyDeckLayout, MoreActionsSheet, MoreTile, NO_SIGNAL_CODE, RailDeckButton, TAKE_OFF, bottomDeck, deckIds, guidedIcon, holdTakeoffHeight, railDeck, takeoffLabel } from './FlyDeck';
import { LocalFlyScreenState, OpenOnRequest } from './FlyScreenState';
import { AnimatedVisibility, useElementSize } from './FlyShell';
import { FLY_STATE, type Readiness, flyState, guidedReadiness, readinessWarning, useHasVehicle, vehicleSubtitle } from './FlyState';
import { GRIPPER_ACTIONS, GripperPanel, gripperOffers } from './GripperPanel';
import { AUTO_INTERCEPT, type AutoIntercept, GUIDED_ACTIONS, type GuidedOffer, type InterceptAction, type Payload, SINGLE_GRIPPER, autoIntercept, guidedOffers, payloadOf, resumeFromSequence } from './GuidedActions';
import { GUIDED_TAKEOFF, guidedTakeoff, guidedTakeoffPath } from './GuidedTakeoff';
import { type GuidedValueKind, GuidedValueFlow, type OpenGuidedValue, altitudeValue, openGuidedValue, speedValue, takeoffValue } from './GuidedValueFlow';
import { holdLabel } from './HoldToConfirm';
import {
  INSTRUMENTS_VIEW,
  INSTRUMENT_GROUPS,
  type InstrumentFact,
  type InstrumentGroup,
  emptyCatalogueText,
  instrumentChoiceNote,
  instrumentGroups,
  instrumentVehicleClass,
  instrumentsPath,
  movedInstrument,
  readChosen,
  readValueSize,
  removedInstrument,
  replacedInstrument,
  selectionId,
  shownFirst,
  showsInstruments,
  vehicleOwnGroup,
  withInstrument,
  writeChosen,
} from './InstrumentChoice';
import { ValueDisplay, ValueDisplayDialog, ValueLabel, displayColour, readDisplays, writeDisplay } from './InstrumentDisplay';
import { MAP_CLICK_PATH, type LoiterOffer, LoiterRadiusPanel, loiterOffer, mapClickUnits } from './MapClickMenu';
import { FlyViewMavlinkActions } from './MavlinkActions';
import { AUTO_POPUP_ACTIONS, GRIPPER_RELEASE, PAUSE, RELEASE, autoMissionPopup, guidedCommand, moreActions, popupReplacesOpenConfirm } from './MoreActions';
import { OFFLINE_SHEET, openAddLink } from './OfflineStatusSheet';
import { ParameterForm } from './ParameterForm';
import { PREFLIGHT, checklistOffered, preflightOffered } from './Preflight';
import { PreflightChecklist, PreflightChecklistReset, useRememberPreflightChecklist } from './PreflightChecklistGate';
import { FLIGHT_MODES_PAGE } from './SetupPages';
import { FootNote, SectionHeader } from './SetupRow';
import { useHasSetupPage } from './SetupScreen';
import { ARM_REQUEST, deckRequestRefusal } from './VehicleStatusSheet';
import { TAP_LOWER } from '@/kotlin/Platform';

const MISSION_POPUP_DELAY_MS = 1000;
const GCS_POSITION = 'view.gcsPosition';

export type ConfirmOption = { readonly label: string; readonly run: (checked: boolean) => void };

export const ConfirmOption = (fields: { label: string; run: (checked: boolean) => void }): ConfirmOption => ({ ...fields });

export type GuidedAction = {
  readonly name: string;
  readonly confirm: string;
  readonly destructive: boolean;
  readonly option: ConfirmOption | null;
  readonly offerId: string | null;
  readonly answer: (() => Promise<string | null>) | null;
  readonly run: () => void;
};

export const GuidedAction = (fields: { name: string; confirm: string; destructive: boolean; option?: ConfirmOption | null; offerId?: string | null; answer?: (() => Promise<string | null>) | null; run?: () => void }): GuidedAction => ({
  option: null,
  offerId: null,
  answer: null,
  run: () => undefined,
  ...fields,
});

export const needsConfirm = (action: GuidedAction, offers: ReadonlyMap<string, GuidedOffer>): boolean =>
  (action.offerId === null ? undefined : offers.get(action.offerId))?.confirm !== false;

export const CHECKLIST_PASSED = 1;
export const CHECKLIST_FAILED = 2;

export const checklistStateValue = (passed: boolean): number => (passed ? CHECKLIST_PASSED : CHECKLIST_FAILED);

export const offerWithdrawn = (offerId: string | null, offers: ReadonlyMap<string, GuidedOffer>): boolean =>
  offerId !== null && offers.get(offerId)?.shown !== true;

export type Instrument = {
  readonly label: string;
  readonly reading: string;
  readonly id: string;
  readonly value: string;
  readonly units: string;
  readonly raw: number | null;
  readonly defaultIcon: string;
};

export const Instrument = (fields: { label: string; reading: string; id?: string; value?: string; units?: string; raw?: number | null; defaultIcon?: string }): Instrument => ({
  id: '',
  value: fields.reading,
  units: '',
  raw: null,
  defaultIcon: '',
  ...fields,
});

export const displayFor = (displays: ReadonlyMap<string, ValueDisplay>, instrument: Instrument): ValueDisplay =>
  displays.get(instrument.id) ?? ValueDisplay({ icon: instrument.defaultIcon });

export const rowWidth = (count: number): number => (count <= 4 ? count : Math.trunc((count + 1) / 2));

const substringBeforeLast = (text: string, delimiter: string): string => {
  const at = text.lastIndexOf(delimiter);
  return at < 0 ? text : text.slice(0, at);
};

const substringAfterLast = (text: string, delimiter: string, missing: string): string => {
  const at = text.lastIndexOf(delimiter);
  return at < 0 ? missing : text.slice(at + delimiter.length);
};

export const operatorDistance = (view: Json | null): readonly Instrument[] => {
  const text = view === null ? '' : optText(view, 'distanceToVehicleText');
  return text.trim() === '' ? [] : [Instrument({ label: 'From you', reading: text, value: substringBeforeLast(text, ' '), units: substringAfterLast(text, ' ', '') })];
};

export const AWAITING_READING = '—';

export const instrumentReading = (item: Json): string | null => {
  if (!optBoolean(item, 'missing')) {
    const units = optText(item, 'units');
    const value = optText(item, 'value');
    return units.trim() === '' ? value : `${value} ${units}`;
  }
  return optText(item, 'missingReason') === 'notReported' ? AWAITING_READING : null;
};

const rawReading = (item: Json): number | null => {
  if (isNull(item, 'raw')) return null;
  const raw = optDouble(item, 'raw');
  return Number.isNaN(raw) ? null : raw;
};

export const instruments = (view: Json | null): readonly Instrument[] =>
  jsonObjects(optJSONArray(view, 'items')).flatMap((item) => {
    const reading = instrumentReading(item);
    return reading === null
      ? []
      : [
          Instrument({
            label: optText(item, 'label'),
            reading,
            id: optText(item, 'id'),
            value: optBoolean(item, 'missing') ? reading : optText(item, 'value'),
            units: optBoolean(item, 'missing') ? '' : optText(item, 'units'),
            raw: rawReading(item),
            defaultIcon: optText(item, 'defaultIcon'),
          }),
        ];
  });

export const VehicleTitle = () => {
  const json = useQgcPath(FLY_STATE);
  const fly = useMemo(() => flyState(json), [json]);
  const communicationLost = fly?.contactLost === true;
  return (
    <div className="flex flex-col">
      <Text typography="titleMedium">Aircast</Text>
      <Text typography="bodySmall" fontWeight={communicationLost ? 700 : 400} color={communicationLost ? md('error') : md('onSurfaceVariant')}>
        {vehicleSubtitle(fly)}
      </Text>
    </div>
  );
};

const colourOf = (display: ValueDisplay, raw: number | null): string | null => {
  const colour = displayColour(display, raw);
  return colour === null ? null : argb(colour);
};

export const TelemetryRow = ({
  className,
  columns = null,
  valuesShown = true,
  chooser = true,
  compact = false,
  stacked = false,
}: {
  className?: string;
  columns?: number | null;
  valuesShown?: boolean;
  chooser?: boolean;
  compact?: boolean;
  stacked?: boolean;
}) => {
  const flyScreen = useContext(LocalFlyScreenState);
  const classView = useQgcPath(INSTRUMENTS_VIEW);
  const vehicleClass = instrumentVehicleClass(classView);
  const [chosen, setChosen] = useKeyedState<readonly string[]>([vehicleClass], () => readChosen(vehicleClass));
  const [displays, setDisplays] = useKeyedState<ReadonlyMap<string, ValueDisplay>>([vehicleClass], () => readDisplays(vehicleClass));
  const [styling, setStyling] = useState<Instrument | null>(null);
  useEffect(() => {
    flyScreen.layout.valueSize.value = readValueSize(vehicleClass);
  }, [vehicleClass, flyScreen]);
  const view = useQgcPathHoldingLast(instrumentsPath(chosen, vehicleClass));
  const gcsJson = useQgcPath(GCS_POSITION);
  const shown = useMemo(() => [...(showsInstruments(chosen) ? instruments(view) : []), ...operatorDistance(gcsJson)], [view, gcsJson, chosen]);
  const stateJson = useQgcPath(FLY_STATE);
  const stale = useMemo(() => flyState(stateJson)?.staleNotice ?? '', [stateJson]);
  const silent = stale.trim() !== '';
  const choosingReadings = useValue(flyScreen.choosingReadings);
  const valueSize = useValue(flyScreen.layout.valueSize);
  const [replacing, setReplacing] = useState<readonly [number, string] | null>(null);

  const change = (next: readonly string[]) => {
    setChosen(next);
    writeChosen(vehicleClass, next);
    flyScreen.instrumentEdits.value += 1;
    setStyling(null);
  };

  const styledIndex = styling === null ? -1 : chosen.findIndex((it) => selectionId(it) === styling.id);
  const index = styledIndex >= 0 ? styledIndex : null;

  const sheets = (
    <>
      {replacing !== null ? (
        <InstrumentSheet
          chosen={chosen}
          title="Change reading"
          onToggle={(path) => {
            const [at, fromId] = replacing;
            const carried = displays.get(fromId);
            if (carried !== undefined && !displays.has(selectionId(path))) {
              writeDisplay(vehicleClass, selectionId(path), carried);
              setDisplays(new Map(displays).set(selectionId(path), carried));
            }
            change(replacedInstrument(chosen, at, path));
            setReplacing(null);
          }}
          onDismiss={() => setReplacing(null)}
        />
      ) : null}
      {styling !== null ? (
        <ValueDisplayDialog
          label={styling.label}
          initial={displayFor(displays, styling)}
          onDismiss={() => setStyling(null)}
          extra={
            index !== null ? (
              <div className="flex flex-wrap gap-1">
                <TextButton
                  onClick={() => {
                    setReplacing([index, styling.id]);
                    setStyling(null);
                  }}
                >
                  Change reading
                </TextButton>
                {index > 0 ? <TextButton onClick={() => change(movedInstrument(chosen, index, -1))}>Move left</TextButton> : null}
                {index < chosen.length - 1 ? <TextButton onClick={() => change(movedInstrument(chosen, index, 1))}>Move right</TextButton> : null}
                <TextButton onClick={() => change(removedInstrument(chosen, index))}>Remove</TextButton>
              </div>
            ) : null
          }
          onDone={(display: ValueDisplay) => {
            writeDisplay(vehicleClass, styling.id, display);
            setDisplays(new Map(displays).set(styling.id, display));
            flyScreen.instrumentEdits.value += 1;
            setStyling(null);
          }}
        />
      ) : null}
      {choosingReadings ? (
        <InstrumentSheet
          chosen={chosen}
          onToggle={(name) => {
            const next = withInstrument(chosen, name);
            setChosen(next);
            writeChosen(vehicleClass, next);
            flyScreen.instrumentEdits.value += 1;
          }}
          onDismiss={() => {
            flyScreen.choosingReadings.value = false;
          }}
        />
      ) : null}
    </>
  );

  if (shown.length === 0) return sheets;

  if (compact) {
    const reading = (instrument: Instrument, typography: Typography) => (
      <div key={`${instrument.id}|${instrument.label}`} className="flex items-baseline gap-1">
        <Text typography="labelMedium" color={withAlpha(aircast('outdoorForeground'), OSD_LABEL_ALPHA)} maxLines={1}>
          {osdLabel(instrument.label)}
        </Text>
        <Text typography={typography} className="tabular-nums" maxLines={1} color={silent ? md('onSurfaceVariant') : colourOf(displayFor(displays, instrument), instrument.raw) ?? aircast('outdoorForeground')}>
          {`${instrument.value}${instrument.units}`}
        </Text>
      </div>
    );
    const speeds = shown.filter((it) => osdIsSpeed(it.label));
    const places = shown.filter((it) => !osdIsSpeed(it.label));
    return (
      <>
        {sheets}
        {stacked ? (
          <div className={`flex flex-col gap-0.5 ${className ?? ''}`}>
            {speeds.length > 0 ? <div className="flex flex-wrap gap-x-4 gap-y-0.5">{speeds.map((it) => reading(it, 'labelLarge'))}</div> : null}
            {places.length > 0 ? <div className="flex flex-wrap gap-x-4 gap-y-0.5">{places.map((it) => reading(it, 'titleLarge'))}</div> : null}
          </div>
        ) : (
          <div className={`flex flex-wrap justify-center gap-x-4 gap-y-0.5 ${className ?? ''}`}>{shown.map((it) => reading(it, 'titleMedium'))}</div>
        )}
      </>
    );
  }

  return (
    <>
      {sheets}
      <AnimatedVisibility visible={valuesShown}>
        <div className="flex flex-col">
          {silent ? <Text typography="bodySmall" color={md('error')} className="block w-full px-2">{stale}</Text> : null}
          <div
            className={`grid w-full justify-between gap-y-1.5 ${className ?? ''}`}
            style={{ gridTemplateColumns: `repeat(${columns ?? rowWidth(shown.length + 1)}, auto)`, opacity: silent ? 0.45 : 1 }}
          >
            {shown.map((instrument) => {
              const display = displayFor(displays, instrument);
              const body = (
                <>
                  <ValueLabel display={display} raw={instrument.raw} label={instrument.label.toUpperCase()} fallback={md('onSurfaceVariant')} />
                  <span className="flex items-baseline gap-[3px]">
                    <span style={{ ...scaledNumber(valueSize.scale), color: colourOf(display, instrument.raw) ?? md('onSurface') }}>{instrument.value}</span>
                    {display.showUnits && instrument.units.trim() !== '' ? <Text typography="labelMedium" color={md('onSurfaceVariant')}>{instrument.units}</Text> : null}
                  </span>
                </>
              );
              const key = `${instrument.id}|${instrument.label}`;
              return instrument.id.trim() === '' ? (
                <div key={key} className="flex flex-col px-3 py-1.5">{body}</div>
              ) : (
                <button key={key} type="button" className="flex flex-col px-3 py-1.5 text-left" onClick={() => setStyling(instrument)}>
                  {body}
                </button>
              );
            })}
            {chooser ? (
              <IconButton
                label="Readings"
                onClick={() => {
                  flyScreen.choosingReadings.value = true;
                }}
              >
                <Drawable name="ic_tune" />
              </IconButton>
            ) : null}
          </div>
        </div>
      </AnimatedVisibility>
    </>
  );
};

export const scaledNumber = (scale: number): CSSProperties => ({
  fontSize: 22 * scale,
  lineHeight: `${26.4 * scale}px`,
  fontWeight: 500,
  fontVariantNumeric: 'tabular-nums',
});

export const FlightActions = ({ className, layout = FlyDeckLayout.Bottom, center = null }: { className?: string; layout?: FlyDeckLayout; center?: ReactNode }) => {
  const flyScreen = useContext(LocalFlyScreenState);
  const deckTelemetry = useContext(LocalDeckTelemetry);
  const stateJson = useQgcPath(FLY_STATE);
  const state = useMemo(() => flyState(stateJson), [stateJson]);
  const readiness = useMemo(() => guidedReadiness(state), [state]);
  const available = state?.connected === true;
  const armed = state?.armed === true;
  const activeId = useRememberActiveVehicleId();
  const [pending, setPending] = useKeyedState<GuidedAction | null>([activeId], () => null);
  const [guidedValue, setGuidedValue] = useKeyedState<OpenGuidedValue | null>([activeId], () => null);
  const [sentName, setSentName] = useState<string | null>(null);
  const [sentSnapshot, setSentSnapshot] = useState<string | null>(null);
  const refusal = useValue(flyScreen.refusal);
  const setRefusal = (message: string | null) => {
    flyScreen.refusal.value = message;
  };
  const [showMore, setShowMore] = useState(false);
  const [showGripper, setShowGripper] = useState(false);
  const [editingLoiter, setEditingLoiter] = useState<LoiterOffer | null>(null);
  const mapClickJson = useQgcPath(MAP_CLICK_PATH);
  const loiter = useMemo(() => loiterOffer(mapClickJson), [mapClickJson]);
  const checklist = useRememberPreflightChecklist();
  const preflightJson = useQgcPath(PREFLIGHT);
  const useChecklist = useMemo(() => preflightOffered(preflightJson), [preflightJson]);
  const actionsJson = useQgcPath(GUIDED_ACTIONS);
  const offers = useMemo(() => guidedOffers(actionsJson), [actionsJson]);
  const interceptJson = useQgcPath(AUTO_INTERCEPT);
  const intercept = useMemo(() => autoIntercept(interceptJson), [interceptJson]);
  const extras = useMemo(() => moreActions(offers), [offers]);
  const gripper = useMemo(() => gripperOffers(extras), [extras]);
  const resumeFrom = useMemo(() => resumeFromSequence(actionsJson), [actionsJson]);
  const payload = useMemo(() => payloadOf(actionsJson), [actionsJson]);
  const liveActions = useMemo(() => (actionsJson === null ? null : JSON.stringify(actionsJson)), [actionsJson]);
  const automaticMissionPopups = useQgcBool(settingControl('settings.flyViewSettings.enableAutomaticMissionPopups'));
  const [missionReady, setMissionReady] = useState<ReadonlySet<string> | null>(null);
  const [popupDue, setPopupDue] = useState<GuidedOffer | null>(null);
  const deckRequest = useValue(flyScreen.deckRequest);

  useLaunchedEffect([offers], () => {
    if (offerWithdrawn(pending?.offerId ?? null, offers)) setPending(null);
    if (offerWithdrawn(guidedValue?.kind.offerId ?? null, offers)) setGuidedValue(null);
    const popup = missionReady === null ? null : autoMissionPopup(missionReady, offers, automaticMissionPopups);
    setMissionReady(new Set(AUTO_POPUP_ACTIONS.filter((it) => offers.get(it)?.ready === true)));
    if (popup !== null) setPopupDue(popup);
  });

  const latestOffers = useUpdatedRef(offers);
  const latestPending = useUpdatedRef(pending);
  const sent = (action: GuidedAction) => {
    const snapshot = liveActions;
    const shown = (name: string | null) => {
      setSentName(name);
      setSentSnapshot(snapshot);
    };
    if (action.answer === null) return shown(action.name);
    void action.answer().then((refused) => {
      setRefusal(refused);
      shown(sentUnlessRefused(action.name, refused));
    });
  };
  const ask = (action: GuidedAction) => {
    if (needsConfirm(action, latestOffers.current)) {
      setPending(action);
    } else {
      action.run();
      sent(action);
    }
  };
  const onIntercept = (picked: InterceptAction) =>
    picked.confirm === null
      ? void Qgc.invoke(picked.path, ...picked.args)
      : ask(GuidedAction({ name: picked.confirm.title, confirm: picked.confirm.text, destructive: false, answer: () => Qgc.refusalOf(picked.path, ...picked.args) }));
  const offerAction = (offer: GuidedOffer, command: () => void): GuidedAction =>
    GuidedAction({ offerId: offer.id, name: offer.title, confirm: offer.prompt, destructive: offer.destructive, run: command });

  useLaunchedEffect([popupDue], async (scope) => {
    const due = popupDue;
    if (due === null) return;
    await delay(MISSION_POPUP_DELAY_MS);
    if (!scope.isActive()) return;
    setPopupDue(null);
    const settled = latestOffers.current.get(due.id);
    if (settled === undefined || !settled.ready) return;
    if (latestPending.current === null || popupReplacesOpenConfirm(settled.id)) {
      const command = guidedCommand(settled.id, resumeFrom);
      if (command !== null) ask(offerAction(settled, command));
    }
  });

  const openValue = (kind: GuidedValueKind) =>
    void (async () => {
      const opened = await openGuidedValue(kind);
      if (opened === null) setRefusal(kind.missingRange);
      else setGuidedValue(opened);
    })();
  const openAltitude = (pauses: boolean) => openValue(altitudeValue(pauses));

  const entries = flightDeckEntries(
    FlightDeckContext({
      offers,
      armed,
      confirm: ask,
      openValue,
      report: setRefusal,
      openChecklist: useChecklist ? () => checklist.open() : null,
      readiness,
      payload,
    }),
  );
  const rail = layout === FlyDeckLayout.Rail;
  const room = useElementSize();
  const centre = useElementSize();
  const offered = deckIds(new Set(entries.map((it) => it.id)), armed);
  const railFit = railDeck(offered, room.size.height);
  const bottomFit = bottomDeck(offered, room.size.width, centre.size.width);
  const deck = rail ? railFit.deck : bottomFit.deck;
  const deckRest = entries.filter((entry) => !deck.some(([id]) => id === entry.id));
  const showingSent = sentIsStillShowing(sentName, sentSnapshot, liveActions);

  useLaunchedEffect([sentName, available], async (scope) => {
    if (!available || sentName === null) return;
    await delay(SENT_SHOWN_MS);
    if (scope.isActive()) setSentName(null);
  });

  useLaunchedEffect([deckRequest, available], () => {
    if (!available) return;
    const asked = deckRequest;
    if (asked === null) return;
    const enabledEntry = entries.find((it) => it.id === asked && it.enabled);
    if (asked === ARM_REQUEST) {
      const stop = armedStopOffer(offers, armed);
      if (enabledEntry !== undefined) enabledEntry.onClick();
      else if (stop !== null) setPending(emergencyStopAction(stop));
      else setRefusal(deckRequestRefusal(offers.get(armed ? 'disarm' : 'arm') ?? null));
    } else if (enabledEntry !== undefined) {
      enabledEntry.onClick();
    } else {
      const offer = offers.get(asked);
      const command = offer?.ready ? guidedCommand(offer.id, resumeFrom) : null;
      if (offer !== undefined && command !== null) ask(offerAction(offer, command));
    }
    flyScreen.deckRequest.value = null;
  });

  useEffect(() => {
    if (available && loiter === null) setEditingLoiter(null);
  }, [loiter === null, available]);

  useEffect(() => {
    if (available && gripper.length === 0) setShowGripper(false);
  }, [gripper.length === 0, available]);

  if (!available) {
    return (
      <>
        <PreflightChecklistReset checklist={checklist} available={available} />
        {!rail ? <OfflineDeck className={className} /> : null}
      </>
    );
  }

  const deciding = pending !== null || guidedValue !== null || editingLoiter !== null;
  const confirming = pending;
  const more = DeckEntry({ id: 'more', label: 'More', icon: 'ic_more_vert', enabled: true, onClick: () => setShowMore(true) });
  const centreBox = (
    <div ref={centre.ref} className="shrink-0 self-center empty:hidden">
      {center}
    </div>
  );
  const confirmTrack =
    confirming !== null ? (
      <ConfirmTrack
        action={confirming}
        onSent={() => {
          sent(confirming);
          setPending(null);
        }}
        onCancel={() => setPending(null)}
      />
    ) : showingSent ? (
      <SentNotice name={sentName ?? ''} onDismiss={() => setSentName(null)} />
    ) : null;
  const refusalLine = refusal !== null ? <RefusalLine message={refusal} onDismiss={() => setRefusal(null)} /> : null;
  const interceptLine = intercept?.visible === true ? <InterceptLine intercept={intercept} onAction={onIntercept} /> : null;
  const decidingCard = refusal !== null || confirming !== null || showingSent;

  const decisions = (
    <>
      {guidedValue !== null ? (
        <DecisionHost rail={rail}>
          <GuidedValueFlow open={guidedValue} onClose={() => setGuidedValue(null)} />
        </DecisionHost>
      ) : null}
      {editingLoiter !== null ? (
        <DecisionHost rail={rail}>
          <LoiterRadiusPanel offer={editingLoiter} units={mapClickUnits(mapClickJson)} onRefused={(message: string) => setRefusal(message)} onDone={() => setEditingLoiter(null)} />
        </DecisionHost>
      ) : null}
    </>
  );

  const body = rail ? (
    <div className={`relative size-full ${className ?? ''}`}>
      {pending === null && guidedValue === null && editingLoiter === null ? (
        <div className="absolute inset-y-0 left-0 flex flex-col" style={{ paddingLeft: 12, paddingBottom: RAIL_LIFT }}>
          <div ref={room.ref} role="group" aria-label={FLIGHT_ACTIONS} className="flex min-h-0 flex-1 flex-col items-center justify-center gap-2">
            {deck.flatMap(([id, primary]) => {
              const entry = entries.find((it) => it.id === id);
              return entry === undefined ? [] : [<RailDeckButton key={id} entry={entry} primary={primary} labelled={armed && railFit.labelled} />];
            })}
            <RailDeckButton entry={more} primary={false} />
          </div>
        </div>
      ) : null}
      {!decidingCard && interceptLine !== null ? (
        <div className="pointer-events-none absolute inset-x-0 bottom-0 flex justify-center">
          <div className="pointer-events-auto w-full p-3" style={{ maxWidth: DECISION_CARD_WIDTH }}>
            <Surface shape="extraLarge" color={md('surfaceContainerLow')}>
              <div className="px-4 py-2">{interceptLine}</div>
            </Surface>
          </div>
        </div>
      ) : null}
      {decidingCard ? (
        <div className="pointer-events-none absolute inset-0 flex items-center justify-center">
          <div className="pointer-events-auto w-full p-3" style={{ maxWidth: DECISION_CARD_WIDTH }}>
            <Surface shape="extraLarge" color={md('surfaceContainerLow')}>
              <div className="flex flex-col gap-3 p-4">
                {interceptLine}
                {refusalLine}
                {confirmTrack}
              </div>
            </Surface>
          </div>
        </div>
      ) : null}
      {decisions}
    </div>
  ) : (
    <>
      <div className={`flex flex-col gap-3 p-4 ${className ?? ''}`}>
        {interceptLine}
        {refusalLine}
        {confirmTrack}
        {deckTelemetry ? (
          <div className="flex w-full justify-center">
            <TelemetryRow valuesShown chooser={false} compact />
          </div>
        ) : null}
        {!deciding && readiness !== null ? <DeckReadiness readiness={readiness} /> : null}
        {pending === null ? (
          <div ref={room.ref} className="flex flex-col gap-3">
            {bottomFit.centreInline ? null : centreBox}
            <DeckRow deck={deck} entries={entries} more={more} center={bottomFit.centreInline ? centreBox : null} />
          </div>
        ) : null}
      </div>
      {decisions}
    </>
  );

  const checklistPast = checklistOffered(armed);
  const stop = armedStopOffer(offers, armed);
  const tiles: readonly MoreTile[] = showMore
    ? [
        ...(stop !== null ? [MoreTile({ label: STOP_MOTORS, icon: 'ic_warning', enabled: true, warning: true, onClick: () => setPending(emergencyStopAction(stop)) })] : []),
        ...deckRest.filter((it) => it.id !== CHECKLIST && it.id !== RELEASE).map((it) => MoreTile({ label: it.label, icon: it.icon, enabled: it.enabled, warning: it.warning, onClick: it.onClick })),
        ...(preflightOffered(preflightJson) ? [MoreTile({ label: 'Checklist', icon: 'ic_check_circle', enabled: checklistPast === null, onClick: () => checklist.open() })] : []),
        ...(loiter !== null ? [MoreTile({ label: loiter.title, icon: 'ic_my_location', enabled: true, onClick: () => setEditingLoiter(loiter) })] : []),
        ...(gripper.length > 0 ? [MoreTile({ label: 'Payload', icon: 'ic_payload', enabled: gripper.some((it: GuidedOffer) => it.ready), onClick: () => setShowGripper(true) })] : []),
        MoreTile({
          label: 'Choose readings',
          icon: 'ic_tune',
          enabled: true,
          onClick: () => {
            flyScreen.choosingReadings.value = true;
          },
        }),
        ...(!armed ? [MoreTile({ label: 'Edit layout', icon: 'ic_edit', enabled: true, onClick: () => flyScreen.layout.startEditing() })] : []),
        ...[intercept?.entry ?? null].flatMap((entry) => (entry === null ? [] : [MoreTile({ label: entry.label, icon: 'ic_search', enabled: true, onClick: () => onIntercept(entry) })])),
        ...extras
          .filter((it) => !entries.some((entry) => entry.id === it.id) && !GRIPPER_ACTIONS.includes(it.id))
          .map((offer) =>
            MoreTile({
              label: offer.title,
              icon: guidedIcon(offer.id),
              enabled: offer.ready,
              warning: offer.destructive,
              onClick: () => {
                if (offer.id === PAUSE) {
                  openAltitude(true);
                } else {
                  const command = guidedCommand(offer.id, resumeFrom);
                  if (command !== null) ask(offerAction(offer, command));
                }
              },
            }),
          ),
      ]
    : [];

  return (
    <>
      <PreflightChecklistReset checklist={checklist} available={available} />
      <PreflightChecklist checklist={checklist} deciding={deciding} />
      <OpenOnRequest name="more" open={() => setShowMore(true)} />
      {body}
      {showMore ? (
        <MoreActionsSheet tiles={tiles} onDismiss={() => setShowMore(false)} footer={<FlyViewMavlinkActions onSent={() => setShowMore(false)} />} />
      ) : null}
      {showGripper && gripper.length > 0 ? <GripperPanel offers={gripper} payload={payload} onDismiss={() => setShowGripper(false)} /> : null}
    </>
  );
};

const DeckRow = ({ deck, entries, more, center }: { deck: DeckIds; entries: readonly DeckEntry[]; more: DeckEntry; center: ReactNode }) => {
  const buttons: readonly (readonly [DeckEntry, boolean])[] = [
    ...deck.flatMap(([id, primary]) => {
      const entry = entries.find((it) => it.id === id);
      return entry === undefined ? [] : [[entry, primary] as const];
    }),
    [more, false] as const,
  ];
  const leading = Math.trunc((buttons.length + 1) / 2);
  const button = ([entry, primary]: readonly [DeckEntry, boolean]) => (
    <DeckButton key={entry.id} entry={entry} primary={primary} buttons={buttons.length} className={entry === more && deck.length > 0 ? 'w-16 shrink-0' : 'min-w-0 flex-1'} />
  );
  return (
    <div role="group" aria-label={FLIGHT_ACTIONS} className="flex items-center gap-[10px]">
      {buttons.slice(0, leading).map(button)}
      {center}
      {buttons.slice(leading).map(button)}
    </div>
  );
};

const OFFLINE_DECK = (): readonly DeckEntry[] => [
  DeckEntry({ id: 'takeoff', label: TAKE_OFF, icon: 'ic_flight_takeoff', enabled: false, onClick: () => undefined }),
  DeckEntry({ id: 'rtl', label: 'Return', icon: 'ic_home', enabled: false, onClick: () => undefined }),
];

const OfflineDeck = ({ className }: { className?: string }) => {
  const navigation = useContext(LocalAppNavigation);
  const flyScreen = useContext(LocalFlyScreenState);
  return (
    <div className={`flex gap-[10px] p-4 ${className ?? ''}`}>
      {OFFLINE_DECK().map((entry) => (
        <div key={entry.id} className="relative min-w-0 flex-1">
          <DeckButton entry={entry} primary={false} className="w-full" />
          <button
            type="button"
            aria-label="Find out how to connect"
            className="absolute inset-0"
            onClick={() => {
              flyScreen.requestedSheet.value = OFFLINE_SHEET;
            }}
          />
        </div>
      ))}
      <DeckButton entry={DeckEntry({ id: 'addLink', label: 'Add a link', icon: 'ic_link', enabled: true, onClick: () => openAddLink(navigation) })} primary className="min-w-0 flex-1" />
    </div>
  );
};

export const LocalDeckTelemetry = createContext(true);

const DECISION_CARD_WIDTH = 460;
const OSD_LABEL_ALPHA = 0.75;
export const STOP_MOTORS = 'Stop motors';
const FLIGHT_ACTIONS = 'Flight actions';
const RAIL_LIFT = 56;

const OSD_LABELS: ReadonlyMap<string, string> = new Map([
  ['distance to home', 'D'],
  ['alt (rel)', 'H'],
  ['altitude', 'H'],
  ['ground speed', 'H.S'],
  ['climb rate', 'V.S'],
  ['air speed', 'A.S'],
  ['airspeed', 'A.S'],
  ['distance to operator', 'D.OP'],
  ['from you', 'D.OP'],
  ['heading', 'HDG'],
  ['distancetohome', 'D'],
  ['altituderelative', 'H'],
  ['groundspeed', 'H.S'],
  ['climbrate', 'V.S'],
]);

export const osdLabel = (label: string): string => OSD_LABELS.get(label.trim().toLowerCase()) ?? label.toUpperCase();

const OSD_SPEEDS: ReadonlySet<string> = new Set(['H.S', 'V.S', 'A.S']);

export const osdIsSpeed = (label: string): boolean => OSD_SPEEDS.has(osdLabel(label));

const DecisionHost = ({ rail, children }: { rail: boolean; children: ReactNode }) =>
  rail ? (
    <div className="pointer-events-none absolute inset-0 flex items-center justify-center p-3">
      <div className="pointer-events-auto w-full" style={{ maxWidth: DECISION_CARD_WIDTH }}>
        {children}
      </div>
    </div>
  ) : (
    <>{children}</>
  );

export const RangeHint = ({ text }: { text: string }) => (
  <Text typography="bodySmall" color={md('onSurfaceVariant')}>
    {text}
  </Text>
);

const flightModeNow = async (): Promise<string> => flyState(await Qgc.get(FLY_STATE))?.mode ?? '';

const InterceptLine = ({ intercept, onAction }: { intercept: AutoIntercept; onAction: (action: InterceptAction) => void }) => (
  <div className="flex w-full flex-wrap items-center gap-2">
    <Text typography="bodyMedium" color={intercept.problem ? md('error') : md('onSurfaceVariant')} className="flex-1">
      {intercept.headline}
    </Text>
    {intercept.actions.map((action) => (
      <TextButton key={action.id} onClick={() => onAction(action)}>
        {action.label}
      </TextButton>
    ))}
  </div>
);

const RefusalLine = ({ message, onDismiss }: { message: string; onDismiss: () => void }) => (
  <div className="flex w-full items-center">
    <Text typography="bodyMedium" color={md('error')} className="flex-1">
      {message}
    </Text>
    <IconButton onClick={onDismiss} label="Dismiss">
      <MaterialIcon name="close" filled />
    </IconButton>
  </div>
);

export const FLIGHT_MODE_SETTINGS_PAGE = 'Flight Mode Settings';

const MENU_ITEM =
  'flex min-h-12 cursor-pointer items-center gap-3 px-3 text-label-large outline-none data-[disabled]:cursor-default data-[disabled]:opacity-40 data-[highlighted]:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)]';

const ModeMenuItem = ({
  text,
  onClick,
  leadingIcon,
  trailingIcon,
  enabled = true,
  className,
}: {
  text: ReactNode;
  onClick: () => void;
  leadingIcon?: ReactNode;
  trailingIcon?: ReactNode;
  enabled?: boolean;
  className?: string;
}) => (
  <Menu.Item closeOnClick={false} disabled={!enabled} onClick={onClick} className={`${MENU_ITEM} ${className ?? ''}`}>
    {leadingIcon ? <span className="flex text-on-surface-variant">{leadingIcon}</span> : null}
    <span className="flex-1">{text}</span>
    {trailingIcon ? <span className="flex text-on-surface-variant">{trailingIcon}</span> : null}
  </Menu.Item>
);

export const FlightModeMenu = ({ expanded, onDismiss, onStatus }: { expanded: boolean; onDismiss: () => void; onStatus: () => void }) => {
  const navigation = useContext(LocalAppNavigation);
  const flyScreen = useContext(LocalFlyScreenState);
  const json = useQgcPath(FLIGHT_MODES);
  const modes = useMemo(() => flightModesView(json), [json]);
  const flyJson = useQgcPath(FLY_STATE);
  const fly = useMemo(() => flyState(flyJson), [flyJson]);
  const actionsJson = useQgcPath(GUIDED_ACTIONS);
  const offered = useMemo(() => guidedOffers(actionsJson).get(TAKEOFF_REQUEST) ?? null, [actionsJson]);
  const takeoff = offered?.shown ? offered : null;
  const [showFolded, setShowFolded] = useState(false);
  const [editing, setEditing] = useState(false);
  const activeId = useRememberActiveVehicleId();
  const [confirming, setConfirming] = useKeyedState<FlightModeOption | null>([activeId], () => null);
  const [settings, setSettings] = useState(false);
  const advanced = useAdvancedUiShown();
  const flightModesPage = useHasSetupPage(FLIGHT_MODES_PAGE);

  if (modes === null) return null;

  const send = (mode: FlightModeOption) =>
    void (async () => {
      flyScreen.pendingMode.value = mode.name;
      const before = modeAck(await Qgc.get(FLIGHT_MODES));
      await VehicleCommands.setFlightMode(mode.name);
      const started = Date.now();
      const settle = async (): Promise<void> => {
        await delay(200);
        const now = modeAck(await Qgc.get(FLIGHT_MODES));
        const reached = (await flightModeNow()) === mode.name;
        return modeOutcome(before, now, reached, Date.now() - started).kind === 'Pending' ? settle() : undefined;
      };
      await settle();
      flyScreen.pendingMode.value = null;
    })();

  const choose = (mode: FlightModeOption) => {
    onDismiss();
    setShowFolded(false);
    if (mode.needsConfirm) setConfirming(mode);
    else send(mode);
  };

  const modeRow = (mode: FlightModeOption) => {
    const caution = mode.caution.trim() === '' ? null : mode.caution;
    const summary = mode.current && !showFolded && mode.summary.trim() !== '' ? mode.summary : null;
    return (
      <ModeMenuItem
        key={mode.name}
        className={mode.current ? 'bg-secondary-container' : undefined}
        leadingIcon={<Drawable name={flightModeIcon(mode.name)} />}
        trailingIcon={mode.current ? <MaterialIcon name="check" filled label="Current mode" className="text-primary" /> : undefined}
        text={
          <span className="flex flex-col" style={{ maxWidth: MODE_TEXT_WIDTH }}>
            <Text typography="titleSmall">{mode.name}</Text>
            {caution !== null ? (
              <Text typography="labelSmall" color={aircast('warning')}>{caution}</Text>
            ) : summary !== null ? (
              <Text typography="labelSmall" color={md('onSurfaceVariant')}>{summary}</Text>
            ) : null}
          </span>
        }
        enabled={modes.canSet}
        onClick={() => (!mode.current ? choose(mode) : onDismiss())}
      />
    );
  };
  const setting = modes.pinnedSetting;
  const warning = readinessWarning(fly);

  return (
    <>
      {settings ? (
        <AircastSheet onDismissRequest={() => setSettings(false)}>
          <ParameterForm page={FLIGHT_MODE_SETTINGS_PAGE} />
          {advanced && flightModesPage ? (
            <TextButton
              onClick={() => {
                setSettings(false);
                navigation.setupPage.value = FLIGHT_MODES_PAGE;
              }}
              className="m-2"
            >
              Configure flight modes
            </TextButton>
          ) : null}
        </AircastSheet>
      ) : null}
      {confirming !== null ? (
        <AlertDialog
          onDismissRequest={() => setConfirming(null)}
          title="Set flight mode"
          text={`Set the vehicle flight mode to ${confirming.name}`}
          confirmButton={
            <TextButton
              onClick={() => {
                setConfirming(null);
                send(confirming);
              }}
            >
              Confirm
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setConfirming(null)}>Cancel</TextButton>}
        />
      ) : null}
      <DropdownMenu
        expanded={expanded}
        onDismissRequest={() => {
          onDismiss();
          setShowFolded(false);
          setEditing(false);
        }}
        className="rounded-shape-lg!"
      >
        {[modes.cannotSetNotice, modes.unknownModeNotice]
          .filter((it) => it.trim() !== '')
          .map((notice) => (
            <div key={notice} className="px-4 py-2" style={{ maxWidth: MODE_TEXT_WIDTH + 32 }}>
              <Text typography="labelSmall" color={aircast('warning')}>{notice}</Text>
            </div>
          ))}
        {!showFolded ? (
          <>
            {takeoff !== null ? (
              <>
                <ModeMenuItem
                  leadingIcon={<Drawable name="ic_flight_takeoff" />}
                  text={
                    <span className="flex flex-col" style={{ maxWidth: MODE_TEXT_WIDTH }}>
                      <Text typography="titleSmall">{takeoff.title.trim() === '' ? TAKE_OFF : takeoff.title}</Text>
                      {!takeoff.ready && takeoff.reason.trim() !== '' ? <Text typography="labelSmall" color={md('onSurfaceVariant')}>{takeoff.reason}</Text> : null}
                    </span>
                  }
                  enabled={takeoff.ready}
                  onClick={() => {
                    onDismiss();
                    flyScreen.deckRequest.value = TAKEOFF_REQUEST;
                  }}
                />
                <HorizontalDivider />
              </>
            ) : null}
            {(() => {
              const now = modes.all.find((it: FlightModeOption) => it.current && !it.quick);
              return now === undefined ? null : modeRow(now);
            })()}
            {modes.quick.map((mode: FlightModeOption, index: number) => (
              <Fragment key={mode.name}>
                {startsSection(modes.quick, index) ? <HorizontalDivider /> : null}
                {modeRow(mode)}
              </Fragment>
            ))}
            <HorizontalDivider />
            <ModeMenuItem text={ALL_MODES} trailingIcon={<MaterialIcon name="keyboard_arrow_right" filled />} onClick={() => setShowFolded(true)} />
          </>
        ) : (
          <>
            <div className="flex items-center pl-4 pr-1" style={{ maxWidth: MODE_TEXT_WIDTH + 36 }}>
              <Text typography="labelSmall" color={md('onSurfaceVariant')} className="flex-1 py-2">
                {editing ? QUICK_LIST_HINT : modeHeading(modes) ?? ALL_MODES}
              </Text>
              {setting !== null ? <TextButton onClick={() => setEditing(!editing)}>{editing ? 'Done' : 'Edit'}</TextButton> : null}
            </div>
            {editing && setting !== null
              ? modes.all.map((mode: FlightModeOption) => (
                  <ModeMenuItem
                    key={mode.name}
                    text={<Text typography="bodyMedium">{mode.name}</Text>}
                    trailingIcon={<Switch checked={mode.quick} onCheckedChange={null} />}
                    onClick={() => {
                      const value = pinsAfter(modes.quick.map((it: FlightModeOption) => it.name), mode.name, !mode.quick);
                      void Qgc.set(setting, value);
                    }}
                  />
                ))
              : modes.all.map((mode: FlightModeOption, index: number) => (
                  <Fragment key={mode.name}>
                    {startsSection(modes.all, index) ? <HorizontalDivider /> : null}
                    {modeRow(mode)}
                  </Fragment>
                ))}
            <HorizontalDivider />
            <ModeMenuItem
              text="Flight mode settings"
              onClick={() => {
                onDismiss();
                setSettings(true);
              }}
            />
          </>
        )}
        <HorizontalDivider />
        <ModeMenuItem
          text={<Text typography="bodyMedium">{VEHICLE_STATUS}</Text>}
          leadingIcon={warning !== null ? <Drawable name="ic_warning" style={{ color: aircast('warning') }} /> : <Drawable name="ic_check_circle" />}
          trailingIcon={<MaterialIcon name="keyboard_arrow_right" filled />}
          onClick={() => {
            onDismiss();
            onStatus();
          }}
        />
      </DropdownMenu>
    </>
  );
};

const MODE_TEXT_WIDTH = 260;
export const ALL_MODES = 'All modes';
export const TAKEOFF_REQUEST = 'takeoff';
const QUICK_LIST_HINT = `Shown first when you ${TAP_LOWER} the mode`;

const InstrumentSheet = ({
  chosen,
  title = 'Instrument tiles',
  onToggle,
  onDismiss,
}: {
  chosen: readonly string[];
  title?: string;
  onToggle: (path: string) => void;
  onDismiss: () => void;
}) => {
  const [groups, setGroups] = useState<readonly InstrumentGroup[]>([]);
  const [shownAtOpen] = useState(chosen);
  const connected = useHasVehicle();
  useLaunchedEffect([connected], async (scope) => {
    const catalogue = await Qgc.get(INSTRUMENT_GROUPS);
    const own = vehicleOwnGroup(catalogue);
    if (scope.isActive()) setGroups([...(own !== null ? [own] : []), ...instrumentGroups(catalogue)]);
  });
  return (
    <AircastSheet onDismissRequest={onDismiss}>
      <Text typography="titleLarge" className="block px-4">{title}</Text>
      <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="block px-4 pb-2">{instrumentChoiceNote(chosen)}</Text>
      {groups.length === 0 ? (
        <FootNote text={emptyCatalogueText(connected)} />
      ) : (
        <div className="w-full overflow-auto">
          {shownFirst(groups, shownAtOpen).map((group: InstrumentGroup) => (
            <Fragment key={`head${group.group}`}>
              <SectionHeader text={sentenceCase(group.title)} />
              {group.facts.map((fact: InstrumentFact) => (
                <ListItem
                  key={fact.path}
                  headlineContent={sentenceCase(fact.label)}
                  leadingContent={<Checkbox checked={chosen.includes(fact.path)} onCheckedChange={null} />}
                  containerColor="transparent"
                  onClick={() => onToggle(fact.path)}
                />
              ))}
            </Fragment>
          ))}
        </div>
      )}
    </AircastSheet>
  );
};

const FLIGHT_MODE_ICONS: readonly (readonly [readonly string[], DrawableName])[] = [
  [['rtl', 'return', 'smart_rtl', 'smartrtl'], 'ic_home'],
  [['land', 'precland'], 'ic_flight_land'],
  [['takeoff'], 'ic_flight_takeoff'],
  [['auto', 'mission'], 'ic_route'],
  [['guided', 'offboard', 'follow'], 'ic_location_on'],
  [['althold', 'altitude', 'alt'], 'ic_height'],
  [['loiter', 'position', 'poshold', 'hold', 'brake'], 'ic_my_location'],
  [['stabilize', 'stabilized', 'manual', 'acro', 'sport', 'drift'], 'ic_gamepad'],
];

export const flightModeIcon = (name: string): DrawableName => {
  const words = name.toLowerCase().split(/[ _-]/);
  const joined = words.join('');
  return FLIGHT_MODE_ICONS.find(([keys]) => keys.some((it) => it === joined || words.includes(it)))?.[1] ?? 'ic_flight';
};

const DeckReadiness = ({ readiness }: { readiness: Readiness }) => (
  <Surface
    color={readiness.blocks ? md('errorContainer') : aircast('warningContainer')}
    contentColor={readiness.blocks ? md('onErrorContainer') : md('onSurface')}
    shape="medium"
    className="w-full"
  >
    <div className="flex items-center gap-[10px] px-3 py-2">
      <Drawable name="ic_warning" size={20} className="shrink-0" style={{ color: readiness.blocks ? md('error') : aircast('warning') }} />
      <Text typography="bodyMedium" maxLines={2}>{readiness.text}</Text>
    </div>
  </Surface>
);

export const VEHICLE_STATUS = 'Vehicle status';

export type FlightDeckContext = {
  readonly offers: ReadonlyMap<string, GuidedOffer>;
  readonly armed: boolean;
  readonly confirm: (action: GuidedAction) => void;
  readonly openValue: (kind: GuidedValueKind) => void;
  readonly report: (message: string | null) => void;
  readonly openChecklist: (() => void) | null;
  readonly readiness: Readiness | null;
  readonly payload: Payload;
};

export const FlightDeckContext = (fields: Omit<FlightDeckContext, 'readiness' | 'payload'> & { readiness?: Readiness | null; payload?: Payload }): FlightDeckContext => ({
  readiness: null,
  payload: SINGLE_GRIPPER,
  ...fields,
});


const dropLabel = (title: string, payload: Payload): string => holdLabel(payload.label.trim() === '' ? title : payload.label);

const promptOr = (offer: GuidedOffer | undefined, fallback: string): string => {
  const prompt = offer?.prompt ?? '';
  return prompt.trim() === '' ? fallback : prompt;
};

const takeoffHoldRefusal = async (offers: ReadonlyMap<string, GuidedOffer>): Promise<string | null> => {
  const heightless = offers.get('takeoff')?.carriesValue === false;
  if (heightless) return VehicleCommands.takeoffRefusal();
  const height = holdTakeoffHeight(guidedTakeoff(await Qgc.get(GUIDED_TAKEOFF)));
  const target = height === null ? null : guidedTakeoff(await Qgc.get(guidedTakeoffPath(height)));
  return target === null ? 'This vehicle did not report a takeoff height range.' : VehicleCommands.takeoffRefusal(target.targetMeters);
};

export const flightDeckEntries = (deck: FlightDeckContext): readonly DeckEntry[] => {
  const { offers, armed, confirm, openValue, report, openChecklist, readiness, payload } = deck;
  const dropBay = payload.next ?? 0;
  const armAction = offers.get(armed ? 'disarm' : 'arm');
  const takeoff = offers.get('takeoff');
  const pause = offers.get(PAUSE);
  const rtl = offers.get('rtl');
  const land = offers.get('land');
  const speed = offers.get('changeSpeed');
  const altitude = offers.get('changeAltitude');
  const release = offers.get(RELEASE);
  const rtlOption = rtl?.option.trim() ? rtl.option : null;
  return [
    armAction?.shown === true
      ? DeckEntry({
          id: 'arm',
          label: armAction.title,
          icon: 'ic_bolt',
          enabled: armAction.ready,
          warning: armed,
          onClick: () =>
            confirm(
              GuidedAction({
                offerId: armed ? 'disarm' : 'arm',
                name: armAction.title,
                confirm: promptOr(armAction, armed ? 'Disarm the vehicle' : 'Arm the vehicle.'),
                destructive: armAction.destructive,
                run: () => void VehicleCommands.setArmed(!armed),
              }),
            ),
        })
      : null,
    takeoff !== undefined && (takeoff.shown || takeoff.reasonCode === NO_SIGNAL_CODE)
      ? DeckEntry({
          id: 'takeoff',
          label: takeoffLabel(takeoff, readiness),
          icon: 'ic_flight_takeoff',
          enabled: takeoff.ready,
          onHold: readiness !== null ? null : () => void takeoffHoldRefusal(offers).then(report),
          onClick: () =>
            takeoff.carriesValue === false
              ? confirm(
                  GuidedAction({
                    offerId: 'takeoff',
                    name: takeoff.title,
                    confirm: promptOr(takeoff, 'Takeoff from ground and hold position.'),
                    destructive: false,
                    run: () => void VehicleCommands.takeoff(),
                  }),
                )
              : openValue(takeoffValue(takeoff)),
        })
      : null,
    pause?.shown === true ? DeckEntry({ id: PAUSE, label: pause.title, icon: 'ic_pause', enabled: pause.ready, onClick: () => openValue(altitudeValue(true)) }) : null,
    rtl?.shown === true
      ? DeckEntry({
          id: 'rtl',
          label: 'Return',
          icon: 'ic_home',
          enabled: rtl.ready,
          onClick: () =>
            confirm(
              GuidedAction({
                offerId: 'rtl',
                name: rtl.title,
                confirm: promptOr(rtl, 'Return to the launch position of the vehicle'),
                destructive: false,
                option: rtlOption === null ? null : ConfirmOption({ label: rtlOption, run: (smart) => void VehicleCommands.returnToLaunch(smart) }),
                run: () => void VehicleCommands.returnToLaunch(false),
              }),
            ),
        })
      : null,
    land?.shown === true
      ? DeckEntry({
          id: 'land',
          label: holdLabel(land.title),
          icon: 'ic_flight_land',
          enabled: land.ready,
          onHold: () => void VehicleCommands.landRefusal().then(report),
          onClick: () =>
            confirm(
              GuidedAction({
                offerId: 'land',
                name: land.title,
                confirm: promptOr(land, 'Land the vehicle at the current position'),
                destructive: false,
                run: () => void VehicleCommands.land(),
              }),
            ),
        })
      : null,
    speed?.shown === true ? DeckEntry({ id: 'changeSpeed', label: 'Speed', icon: 'ic_speed', enabled: speed.ready, onClick: () => openValue(speedValue(speed)) }) : null,
    altitude?.shown === true ? DeckEntry({ id: 'changeAltitude', label: 'Altitude', icon: 'ic_height', enabled: altitude.ready, onClick: () => openValue(altitudeValue(false)) }) : null,
    release?.shown === true
      ? DeckEntry({
          id: RELEASE,
          label: dropLabel(release.title, payload),
          icon: 'ic_drop',
          enabled: release.ready,
          onHold: () => void VehicleCommands.gripperRefusal(GRIPPER_RELEASE, dropBay).then(report),
          onClick: () =>
            confirm(
              GuidedAction({
                offerId: RELEASE,
                name: release.title,
                confirm: promptOr(release, 'Open the gripper and drop the payload.'),
                destructive: false,
                answer: () => VehicleCommands.gripperRefusal(GRIPPER_RELEASE, dropBay),
              }),
            ),
        })
      : null,
    openChecklist !== null && checklistOffered(armed) === null ? DeckEntry({ id: CHECKLIST, label: 'Checklist', icon: 'ic_check_circle', enabled: true, onClick: () => openChecklist() }) : null,
  ].filter((it): it is DeckEntry => it !== null);
};

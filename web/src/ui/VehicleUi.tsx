import { type CSSProperties, Fragment, type ReactNode, createContext, useContext, useEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
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
import { AircastSpace, aircast, argb, md, withAlpha } from '@/theme/Colors';
import { useRememberActiveVehicleId } from './ActiveVehicle';
import { useAdvancedUiShown } from './AdvancedUi';
import { LocalAppNavigation } from './AppNavigation';
import { ConfirmTrack, SENT_SHOWN_MS, SentNotice, sentIsStillShowing, sentUnlessRefused } from './ConfirmTrack';
import { armedStopOffer, emergencyStopAction } from './EmergencyStop';
import { FLIGHT_MODES, type FlightModeOption, flightModesView, modeAck, modeConfirmText, modeHeading, modeOutcome, pinsAfter, startsSection } from './FlightModes';
import { CHECKLIST, DeckButton, DeckEntry, type DeckIds, FlyDeckLayout, FlyMenuSheet, MenuTile, NO_SIGNAL_CODE, RAIL_STOP_GAP, RETURN_HOME, RailDeckButton, TAKE_OFF, bottomDeck, deckIds, guidedIcon, holdTakeoffHeight, railDeck, takeoffLabel, useFlyDestinations } from './FlyDeck';
import { LocalFlyScreenState, MENU_SHEET, OpenOnRequest, STATUS_ALL_SHEET, useVehicleScoped } from './FlyScreenState';
import { AnimatedVisibility, PASS_THROUGH, useElementSize, useFlyIsPortrait } from './FlyShell';
import { FLY_STATE, type Readiness, STALE_OPACITY, flyState, useGuidedReadiness, useHasVehicle, useStaleNotice, vehicleSubtitle } from './FlyState';
import { GRIPPER_ACTIONS, GripperPanel, gripperOffers } from './GripperPanel';
import { AUTO_PURSUIT, type AutoPursuit, GUIDED_ACTIONS, type GuidedOffer, type PursuitAction, type Payload, SINGLE_GRIPPER, autoPursuit, guidedOffers, payloadOf, resumeFromSequence } from './GuidedActions';
import { GUIDED_TAKEOFF, guidedTakeoff, guidedTakeoffPath } from './GuidedTakeoff';
import { type GuidedValueKind, GuidedValueFlow, altitudeValue, openGuidedValue, speedValue, takeoffValue } from './GuidedValueFlow';
import { focusLost, useGuidedPanelShown } from './GuidedValuePanel';
import { holdLabel } from './HoldToConfirm';
import {
  CHOOSE_READINGS,
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
  SEARCH_READINGS,
  noReadingFound,
  removedInstrument,
  replacedInstrument,
  searchedGroups,
  selectionId,
  shownFirst,
  showsInstruments,
  vehicleOwnGroup,
  withInstrument,
  writeChosen,
} from './InstrumentChoice';
import { ValueDisplay, ValueDisplayDialog, ValueIcon, ValueLabel, displayColour, displayIcon, readDisplays, writeDisplay } from './InstrumentDisplay';
import { MAP_CLICK_PATH, type LoiterOffer, LoiterRadiusPanel, loiterOffer, mapClickUnits } from './MapClickMenu';
import { FlyViewMavlinkActions } from './MavlinkActions';
import { AUTO_POPUP_ACTIONS, GRIPPER_RELEASE, PAUSE, RELEASE, autoMissionPopup, guidedCommand, missionFlown, moreActions, popupReplacesOpenConfirm } from './MoreActions';
import { CHECK_DEVICE, OFFLINE_SHEET, OFFLINE_STATUS_VIEW, offlineStatus, openAddLink, openDeviceTelemetry } from './OfflineStatusSheet';
import { LayoutWidget, barOrderWith, homeRows, orientedKey, rowsCovered } from './OverlayLayout';
import { LocalFlyOsd, OSD_SCRIM, osdShadow } from './Osd';
import { ParameterForm } from './ParameterForm';
import { PAYLOAD_RAIL_VIEW, payloadRailOf } from './PayloadRail';
import { PREFLIGHT, checklistOffered, preflightOffered } from './Preflight';
import { PreflightChecklist, PreflightChecklistReset, useRememberPreflightChecklist } from './PreflightChecklistGate';
import { FLIGHT_MODES_PAGE } from './SetupPages';
import { FootNote, SearchPill, SectionHeader } from './SetupRow';
import { useHasSetupPage } from './SetupScreen';
import { ARM_REQUEST, deckRequestRefusal } from './VehicleStatusSheet';
import { TAP_LOWER } from '@/kotlin/Platform';
import { withUnit } from './Quantity';

const MISSION_POPUP_DELAY_MS = 1000;
const GCS_POSITION = 'view.gcsPosition';

export type ConfirmOption = { readonly label: string; readonly hint: string; readonly run: (checked: boolean) => void };

export const ConfirmOption = (fields: { label: string; hint?: string; run: (checked: boolean) => void }): ConfirmOption => ({ hint: '', ...fields });

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
  readonly groupIcon: string;
  readonly tag: string;
};

export const Instrument = (fields: { label: string; reading: string; id?: string; value?: string; units?: string; raw?: number | null; defaultIcon?: string; groupIcon?: string; tag?: string }): Instrument => ({
  id: '',
  value: fields.reading,
  units: '',
  raw: null,
  defaultIcon: '',
  groupIcon: '',
  tag: '',
  ...fields,
});

const ownGlyph = (instrument: Instrument): string => (instrument.defaultIcon.trim() !== '' ? instrument.defaultIcon : instrument.groupIcon);

export const displayFor = (displays: ReadonlyMap<string, ValueDisplay>, instrument: Instrument): ValueDisplay =>
  displays.get(instrument.id) ?? ValueDisplay({ icon: ownGlyph(instrument) });

export type HudSide = 'start' | 'end';

export const readingKey = (id: string): string => `reading:${id}`;

export const instrumentKey = (instrument: Instrument): string => readingKey(instrument.id.trim() === '' ? instrument.label : instrument.id);

export const hudSide = (instrument: Instrument): HudSide => (osdIsSpeed(instrument.label) ? 'end' : 'start');

export const HUD_READING_HEIGHT = 24;

const HUD_LABEL_HEIGHT = 20;

export const HUD_READING_GAP = 2;

export const hudReadingHeight = (scale: number): number => Math.max(HUD_READING_HEIGHT * scale, HUD_LABEL_HEIGHT);

export const hudIcon = (display: ValueDisplay, instrument: Instrument): string | null =>
  displayIcon(display, instrument.raw) ?? (display.text.trim() === '' && ownGlyph(instrument).trim() !== '' ? ownGlyph(instrument) : null);

export const hudTag = (display: ValueDisplay, instrument: Instrument): string | null =>
  displayIcon(display, instrument.raw) === null && display.text.trim() === '' && instrument.defaultIcon.trim() === '' && instrument.groupIcon.trim() !== '' && instrument.tag.trim() !== ''
    ? instrument.tag.toUpperCase()
    : null;

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
    return withUnit(value, units);
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
            groupIcon: optText(item, 'groupIcon'),
            tag: optText(item, 'tag'),
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

const useTelemetryReadings = () => {
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
  const stale = useStaleNotice();
  const silent = stale !== '';
  const choosingReadings = useValue(flyScreen.choosingReadings);
  const valueSize = useValue(flyScreen.layout.valueSize);
  const [replacing, setReplacing] = useState<readonly [number, string] | null>(null);

  const keep = (next: readonly string[]) => {
    flyScreen.layout.forget(chosen.filter((it) => !next.includes(it)).map((it) => readingKey(selectionId(it))));
    setChosen(next);
    writeChosen(vehicleClass, next);
    flyScreen.instrumentEdits.value += 1;
  };

  const change = (next: readonly string[]) => {
    keep(next);
    setStyling(null);
  };

  const removal = (it: Instrument): (() => void) | undefined => (chosen.some((path) => selectionId(path) === it.id) ? () => keep(chosen.filter((path) => selectionId(path) !== it.id)) : undefined);

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
          onToggle={(name) => keep(withInstrument(chosen, name))}
          onDismiss={() => {
            flyScreen.choosingReadings.value = false;
          }}
        />
      ) : null}
    </>
  );

  return { flyScreen, shown, displays, silent, stale, valueSize, sheets, setStyling, removal };
};

export const TelemetryRow = ({
  className,
  columns = null,
  valuesShown = true,
  chooser = true,
  compact = false,
  stacked = false,
  placeable = false,
}: {
  className?: string;
  columns?: number | null;
  valuesShown?: boolean;
  chooser?: boolean;
  compact?: boolean;
  stacked?: boolean;
  placeable?: boolean;
}) => {
  const { flyScreen, shown, displays, silent, stale, valueSize, sheets, setStyling } = useTelemetryReadings();
  if (shown.length === 0) return sheets;

  if (compact) {
    const shownReading = (instrument: Instrument, typography: Typography) => (
      <div className="flex items-baseline gap-1" style={{ opacity: silent ? STALE_OPACITY : 1 }}>
        <Text typography="labelLarge" color={withAlpha(aircast('outdoorForeground'), OSD_LABEL_ALPHA)} maxLines={1}>
          {osdLabel(instrument.label)}
        </Text>
        <Text typography={typography} className="tabular-nums" maxLines={1} style={{ zoom: valueSize.scale }} color={silent ? md('onSurfaceVariant') : colourOf(displayFor(displays, instrument), instrument.raw) ?? aircast('outdoorForeground')}>
          {withUnit(instrument.value, instrument.units)}
        </Text>
      </div>
    );
    const reading = (instrument: Instrument, typography: Typography) =>
      placeable ? (
        <LayoutWidget key={instrumentKey(instrument)} widgetKey={instrumentKey(instrument)}>
          {shownReading(instrument, typography)}
        </LayoutWidget>
      ) : (
        <Fragment key={instrumentKey(instrument)}>{shownReading(instrument, typography)}</Fragment>
      );
    const speeds = shown.filter((it) => osdIsSpeed(it.label));
    const places = shown.filter((it) => !osdIsSpeed(it.label));
    const staleNote = silent ? (
      <Text typography="labelMedium" color={md('error')} className="block basis-full">
        {stale}
      </Text>
    ) : null;
    return (
      <>
        {sheets}
        {stacked ? (
          <div className={`flex flex-col gap-0.5 ${className ?? ''}`}>
            {staleNote}
            {speeds.length > 0 ? <div className="flex flex-wrap gap-x-4 gap-y-0.5">{speeds.map((it) => reading(it, 'headlineMedium'))}</div> : null}
            {places.length > 0 ? <div className="flex flex-wrap gap-x-4 gap-y-0.5">{places.map((it) => reading(it, 'displaySmall'))}</div> : null}
          </div>
        ) : (
          <div className={`flex flex-wrap justify-center gap-x-4 gap-y-0.5 ${className ?? ''}`}>
            {staleNote}
            {shown.map((it) => reading(it, 'titleMedium'))}
          </div>
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
            style={{ gridTemplateColumns: `repeat(${columns ?? rowWidth(shown.length + 1)}, auto)`, opacity: silent ? STALE_OPACITY : 1 }}
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

const READING_ACTIONS: ReadonlyMap<string, readonly [string, string]> = new Map([
  ['altitudeRelative', ['changeAltitude', 'change altitude']],
  ['groundSpeed', ['changeSpeed', 'change speed']],
  ['airSpeed', ['changeSpeed', 'change speed']],
]);

export const readingAction = (instrument: Instrument): readonly [string, string] | null => READING_ACTIONS.get(instrument.id.slice(instrument.id.lastIndexOf('/') + 1)) ?? null;

type ReadingAct = { readonly label: string; readonly run: () => void };

const HudReading = ({ instrument, display, silent, scale, act }: { instrument: Instrument; display: ValueDisplay; silent: boolean; scale: number; act: ReadingAct | null }) => {
  const icon = hudIcon(display, instrument);
  const tag = hudTag(display, instrument);
  const tint = silent ? md('onSurfaceVariant') : colourOf(display, instrument.raw) ?? aircast('outdoorForeground');
  const bare = useContext(LocalFlyOsd);
  const Root = act === null ? 'div' : 'button';
  return (
    <Root
      {...(act === null ? {} : { type: 'button' as const, onClick: act.run, 'aria-label': `${osdLabel(instrument.label)} ${withUnit(instrument.value, instrument.units)}, ${act.label}`, title: act.label })}
      className={`flex items-center gap-1.5 ${act === null ? '' : 'cursor-pointer'} ${bare ? '' : 'rounded-shape-sm px-1.5'}`}
      style={{ height: hudReadingHeight(scale), opacity: silent ? STALE_OPACITY : 1, background: bare ? undefined : OSD_SCRIM }}
    >
      {icon !== null ? (
        <>
          <span aria-hidden className="contents">
            <ValueIcon name={icon} tint={tint} size={16} />
          </span>
          <span className="sr-only">{osdLabel(instrument.label)}</span>
          {tag !== null ? (
            <span aria-hidden className="contents">
              <Text typography="labelLarge" color={tint} maxLines={1}>
                {tag}
              </Text>
            </span>
          ) : null}
        </>
      ) : (
        <Text typography="labelLarge" color={tint} maxLines={1}>
          {osdLabel(instrument.label)}
        </Text>
      )}
      <Text typography="titleMedium" fontWeight={700} className="tabular-nums" maxLines={1} style={{ zoom: scale }} color={tint}>
        {withUnit(instrument.value, instrument.units)}
      </Text>
    </Root>
  );
};

export const hudStartTop = (top: CSSProperties['top'], below: number): CSSProperties['top'] => (below > 0 ? `max(${String(top)}, ${below}px)` : top);

export const HudReadings = ({ style, startInset, startBelow = 0, endLift = 0 }: { style: CSSProperties; startInset: number; startBelow?: number; endLift?: number }) => {
  const { flyScreen, shown, displays, silent, stale, valueSize, sheets, removal } = useTelemetryReadings();
  const offsets = useValue(flyScreen.layout.offsets);
  const editing = useValue(flyScreen.layout.editing);
  const docked = useValue(flyScreen.layout.docked);
  const dockSlots = useValue(flyScreen.dockSlots);
  const inBar = (it: Instrument) => docked.includes(instrumentKey(it));
  const dock = (it: Instrument) => (x: number) => {
    flyScreen.layout.saveIndicatorOrder(barOrderWith(instrumentKey(it), x));
    flyScreen.layout.saveDocked([...docked.filter((key) => key !== instrumentKey(it)), instrumentKey(it)]);
  };
  const actionsJson = useQgcPath(GUIDED_ACTIONS);
  const offers = useMemo(() => guidedOffers(actionsJson), [actionsJson]);
  const actFor = (it: Instrument): ReadingAct | null => {
    const action = readingAction(it);
    const offer = action === null ? undefined : offers.get(action[0]);
    return action !== null && offer !== undefined && offer.shown && offer.ready && !editing
      ? {
          label: action[1],
          run: () => {
            flyScreen.deckRequest.value = action[0];
          },
        }
      : null;
  };
  const landscape = !useFlyIsPortrait();
  const height = hudReadingHeight(valueSize.scale);
  const pitch = height + HUD_READING_GAP;
  const noteSide: HudSide = shown.some((it) => hudSide(it) === 'start') ? 'start' : 'end';
  const column = (side: HudSide) => {
    const readings = shown.filter((it) => hudSide(it) === side && !inBar(it));
    const lift = side === 'end' ? endLift : 0;
    const placed = (it: Instrument) => offsets.get(orientedKey(instrumentKey(it), landscape));
    const taken = new Set(readings.map(placed).flatMap((at) => (at === undefined ? [] : rowsCovered([at[0], at[1] + lift], pitch, height))));
    const rows = homeRows(readings.map(instrumentKey), taken);
    const stacked = Math.max(0, ...[...rows.values()].map((row) => row + 1));
    return readings.length === 0 ? null : (
      <div
        className={`absolute ${side === 'start' ? 'start-0 items-start' : 'end-0 items-end'} flex flex-col justify-end-safe gap-0.5 ${PASS_THROUGH}`}
        style={{ ...style, ...(side === 'start' ? { paddingInlineStart: startInset, top: hudStartTop(style.top, startBelow) } : { paddingInlineEnd: AircastSpace.s3 }) }}
      >
        {silent && side === noteSide ? (
          <Text typography="labelMedium" color={md('error')} className={`max-w-[220px] ${osdShadow()}`}>
            {stale}
          </Text>
        ) : null}
        <div className="relative shrink-0" style={{ height: Math.max(0, stacked * pitch - HUD_READING_GAP + lift) }}>
          {readings.map((it) => (
            <div key={instrumentKey(it)} className={`absolute bottom-0 w-max ${side === 'start' ? 'start-0' : 'end-0'}`}>
              <LayoutWidget widgetKey={instrumentKey(it)} home={[0, -(rows.get(instrumentKey(it)) ?? 0) * pitch - lift]} badgeBeside={side === 'start' ? 'end' : 'start'} onBar={dock(it)} onRemove={removal(it)} resizable>
                <HudReading instrument={it} display={displayFor(displays, it)} silent={silent} scale={valueSize.scale} act={actFor(it)} />
              </LayoutWidget>
            </div>
          ))}
        </div>
      </div>
    );
  };
  return (
    <>
      {sheets}
      {column('start')}
      {column('end')}
      {shown.filter(inBar).map((it) => {
        const slot = dockSlots.get(instrumentKey(it));
        return slot === undefined ? null : createPortal(<HudReading instrument={it} display={displayFor(displays, it)} silent={silent} scale={1} act={actFor(it)} />, slot, instrumentKey(it));
      })}
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
  const panelOpen = useValue(flyScreen.guidedPanels) > 0;
  const destinations = useFlyDestinations();
  const deckTelemetry = useContext(LocalDeckTelemetry);
  const stateJson = useQgcPath(FLY_STATE);
  const state = useMemo(() => flyState(stateJson), [stateJson]);
  const readiness = useGuidedReadiness(state);
  const available = state?.connected === true;
  const armed = state?.armed === true;
  const activeId = useRememberActiveVehicleId();
  const [pending, setPending] = useVehicleScoped(flyScreen.confirming, activeId);
  const [guidedValue, setGuidedValue] = useVehicleScoped(flyScreen.choosingValue, activeId);
  const [sentName, setSentName] = useState<string | null>(null);
  const [sentSnapshot, setSentSnapshot] = useState<string | null>(null);
  const refusal = useValue(flyScreen.refusal);
  const setRefusal = (message: string | null) => {
    flyScreen.refusal.value = message;
  };
  const [menuOpen, setMenuOpen] = useState(false);
  const [showGripper, setShowGripper] = useState(false);
  const [editingLoiter, setEditingLoiter] = useState<LoiterOffer | null>(null);
  const mapClickJson = useQgcPath(MAP_CLICK_PATH);
  const loiter = useMemo(() => loiterOffer(mapClickJson), [mapClickJson]);
  const checklist = useRememberPreflightChecklist();
  const preflightJson = useQgcPath(PREFLIGHT);
  const useChecklist = useMemo(() => preflightOffered(preflightJson), [preflightJson]);
  const actionsJson = useQgcPath(GUIDED_ACTIONS);
  const offers = useMemo(() => guidedOffers(actionsJson), [actionsJson]);
  const pursuitJson = useQgcPath(AUTO_PURSUIT);
  const pursuit = useMemo(() => autoPursuit(pursuitJson), [pursuitJson]);
  const extras = useMemo(() => moreActions(offers), [offers]);
  const gripper = useMemo(() => gripperOffers(extras), [extras]);
  const resumeFrom = useMemo(() => resumeFromSequence(actionsJson), [actionsJson]);
  const payload = useMemo(() => payloadOf(actionsJson), [actionsJson]);
  const liveActions = useMemo(() => (actionsJson === null ? null : JSON.stringify(actionsJson)), [actionsJson]);
  const automaticMissionPopups = useQgcBool(settingControl('settings.flyViewSettings.enableAutomaticMissionPopups'));
  const [missionReady, setMissionReady] = useState<ReadonlySet<string> | null>(null);
  const [flownMission, setFlownMission] = useKeyedState([activeId], () => false);
  const [popupDue, setPopupDue] = useState<GuidedOffer | null>(null);
  const deckRequest = useValue(flyScreen.deckRequest);

  useLaunchedEffect([offers], () => {
    if (offerWithdrawn(pending?.offerId ?? null, offers)) setPending(null);
    if (offerWithdrawn(guidedValue?.kind.offerId ?? null, offers)) setGuidedValue(null);
    const flown = missionFlown(flownMission, armed, optBoolean(actionsJson, 'missionAvailable'));
    const popup = missionReady === null ? null : autoMissionPopup(missionReady, offers, automaticMissionPopups, flown);
    setFlownMission(flown);
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
  const onPursuit = (picked: PursuitAction) =>
    picked.confirm === null
      ? void Qgc.invoke(picked.path, ...picked.args)
      : ask(GuidedAction({ name: picked.confirm.title, confirm: picked.confirm.text, destructive: false, answer: () => Qgc.refusalOf(picked.path, ...picked.args) }));
  const offerAction = (offer: GuidedOffer, command: () => Promise<string | null>): GuidedAction =>
    GuidedAction({ offerId: offer.id, name: offer.title, confirm: offer.prompt, destructive: offer.destructive, answer: command });

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
  const payloadStrip = payloadRailOf(useQgcPath(PAYLOAD_RAIL_VIEW)) !== null;
  const offered = deckIds(new Set(entries.map((it) => it.id)), armed, payloadStrip);
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

  const screen: readonly MenuTile[] = [
    MenuTile({
      label: CHOOSE_READINGS,
      icon: 'ic_tune',
      enabled: true,
      onClick: () => {
        flyScreen.choosingReadings.value = true;
      },
    }),
    MenuTile({ label: 'Edit layout', icon: 'ic_edit', enabled: true, onClick: () => flyScreen.layout.startEditing() }),
  ];
  const closeMenu = () => setMenuOpen(false);
  useGuidedPanelShown(available && pending !== null);

  const anyDecision = pending !== null || guidedValue !== null || editingLoiter !== null || panelOpen;
  const decidedBefore = useRef(anyDecision);
  useEffect(() => {
    if (decidedBefore.current && !anyDecision && focusLost()) room.node.current?.querySelector<HTMLElement>('button:not([disabled]), [role="button"]:not([aria-disabled="true"])')?.focus({ preventScroll: true });
    decidedBefore.current = anyDecision;
  }, [anyDecision]);

  if (!available) {
    return (
      <>
        <PreflightChecklistReset checklist={checklist} available={available} />
        <OpenOnRequest name={MENU_SHEET} open={() => setMenuOpen(true)} />
        {!rail ? <OfflineDeck className={className} /> : null}
        {menuOpen ? (
          <FlyMenuSheet
            sections={[
              { title: 'Go to', tiles: destinations },
              { title: 'Screen', tiles: screen },
            ]}
            onDismiss={closeMenu}
            footer={null}
          />
        ) : null}
      </>
    );
  }

  const deciding = anyDecision;
  const confirming = pending;
  const stop = armedStopOffer(offers, armed);
  const stopEntry = stop === null ? null : DeckEntry({ id: 'stopMotors', label: STOP_MOTORS, icon: 'ic_warning', enabled: true, warning: true, onClick: () => setPending(emergencyStopAction(stop)) });
  const centreBox = (
    <div ref={centre.ref} className="shrink-0 self-center empty:hidden has-[[data-layout-item]:empty]:hidden">
      <LayoutWidget widgetKey="deckShutters" movable={false} hideable={false} live={armed}>
        {center}
      </LayoutWidget>
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
  const pursuitLine = pursuit?.visible === true ? <PursuitLine pursuit={pursuit} onAction={onPursuit} /> : null;
  const decidingCard = refusal !== null || confirming !== null || showingSent;

  const decisions = (
    <>
      {guidedValue !== null ? (
        <DecisionHost rail={rail}>
          <GuidedValueFlow open={guidedValue} onClose={() => setGuidedValue(null)} onRefused={setRefusal} />
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
      {!deciding ? (
        <div className="absolute inset-y-0 left-0 flex flex-col" style={{ paddingLeft: 12, paddingBottom: RAIL_LIFT }}>
          <div ref={room.ref} role="group" aria-label={FLIGHT_ACTIONS} className="flex min-h-0 flex-1 flex-col items-center justify-center">
            <div className="flex flex-col items-center gap-2">
              {deck.flatMap(([id, primary]) => {
                const entry = entries.find((it) => it.id === id);
                return entry === undefined
                  ? []
                  : [
                      <LayoutWidget key={id} widgetKey={deckKey(id)} hideable={false} live={armed}>
                        <RailDeckButton entry={entry} primary={primary} labelled={railFit.labelled} />
                      </LayoutWidget>,
                    ];
              })}
              {stopEntry !== null ? (
                <div style={{ marginTop: RAIL_STOP_GAP }}>
                  <LayoutWidget widgetKey={deckKey(stopEntry.id)} hideable={false} live={armed}>
                    <RailDeckButton entry={stopEntry} primary={false} labelled={railFit.labelled} />
                  </LayoutWidget>
                </div>
              ) : null}
            </div>
          </div>
        </div>
      ) : null}
      {!decidingCard && pursuitLine !== null ? (
        <div className="pointer-events-none absolute inset-x-0 bottom-0 flex justify-center">
          <div className="pointer-events-auto w-full p-3" style={{ maxWidth: DECISION_CARD_WIDTH }}>
            <Surface shape="extraLarge" color={md('surfaceContainerLow')}>
              <div className="px-4 py-2">{pursuitLine}</div>
            </Surface>
          </div>
        </div>
      ) : null}
      {decidingCard ? (
        <div className="pointer-events-none absolute inset-0 flex items-center justify-center">
          <div className="pointer-events-auto w-full p-3" style={{ maxWidth: DECISION_CARD_WIDTH }}>
            <Surface shape="extraLarge" color={md('surfaceContainerLow')}>
              <div className="flex flex-col gap-3 p-4">
                {pursuitLine}
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
        {pursuitLine}
        {refusalLine}
        {confirmTrack}
        {deckTelemetry ? (
          <div className="flex w-full justify-center">
            <LayoutWidget widgetKey="telemetry" movable={false} hideable={false}>
              <TelemetryRow valuesShown chooser={false} compact />
            </LayoutWidget>
          </div>
        ) : null}
        {!deciding && readiness !== null ? (
          <LayoutWidget widgetKey="readiness" movable={false} hideable={false}>
            <DeckReadiness readiness={readiness} />
          </LayoutWidget>
        ) : null}
        {!deciding ? (
          <div ref={room.ref}>
            <div className="flex flex-col gap-3">
              {bottomFit.centreInline ? null : centreBox}
              <DeckRow deck={deck} entries={entries} extra={stopEntry} center={bottomFit.centreInline ? centreBox : null} live={armed} />
            </div>
          </div>
        ) : null}
      </div>
      {decisions}
    </>
  );

  const checklistPast = checklistOffered(armed);
  const aircraft: readonly MenuTile[] = menuOpen
    ? [
        MenuTile({
          label: 'All status',
          icon: 'ic_sensors',
          enabled: true,
          onClick: () => {
            flyScreen.requestedSheet.value = STATUS_ALL_SHEET;
          },
        }),
        ...deckRest.filter((it) => it.id !== CHECKLIST && it.id !== RELEASE).map((it) => MenuTile({ label: it.label, icon: it.icon, enabled: it.enabled, warning: it.warning, onClick: it.onClick })),
        ...(preflightOffered(preflightJson) ? [MenuTile({ label: 'Checklist', icon: 'ic_check_circle', enabled: checklistPast === null, onClick: () => checklist.open() })] : []),
        ...(loiter !== null ? [MenuTile({ label: loiter.title, icon: 'ic_my_location', enabled: true, onClick: () => setEditingLoiter(loiter) })] : []),
        ...(gripper.length > 0 ? [MenuTile({ label: 'Payload', icon: 'ic_payload', enabled: gripper.some((it: GuidedOffer) => it.ready), onClick: () => setShowGripper(true) })] : []),
        ...[pursuit?.entry ?? null].flatMap((entry) => (entry === null ? [] : [MenuTile({ label: entry.label, icon: 'ic_search', enabled: true, onClick: () => onPursuit(entry) })])),
        ...extras
          .filter((it) => !entries.some((entry) => entry.id === it.id) && !GRIPPER_ACTIONS.includes(it.id))
          .map((offer) =>
            MenuTile({
              label: offer.title,
              icon: guidedIcon(offer.id),
              enabled: offer.ready,
              onClick: () => {
                if (offer.id === PAUSE) {
                  pauseClick(offer, ask, openValue);
                } else {
                  const command = guidedCommand(offer.id, resumeFrom);
                  if (command !== null) ask(offerAction(offer, command));
                }
              },
            }),
          ),
      ]
    : [];
  const sections = [
    { title: 'Go to', tiles: destinations },
    { title: 'Aircraft', tiles: aircraft },
    { title: 'Screen', tiles: screen },
  ];

  return (
    <>
      <PreflightChecklistReset checklist={checklist} available={available} />
      <PreflightChecklist checklist={checklist} deciding={deciding} />
      <OpenOnRequest name={MENU_SHEET} open={() => setMenuOpen(true)} />
      {body}
      {menuOpen ? (
        <FlyMenuSheet sections={sections} onDismiss={closeMenu} footer={<FlyViewMavlinkActions onSent={closeMenu} />} />
      ) : null}
      {showGripper && gripper.length > 0 ? <GripperPanel offers={gripper} payload={payload} onDismiss={() => setShowGripper(false)} /> : null}
    </>
  );
};

export const deckKey = (id: string): string => `deck:${id}`;

const DeckRow = ({ deck, entries, extra, center, live }: { deck: DeckIds; entries: readonly DeckEntry[]; extra: DeckEntry | null; center: ReactNode; live: boolean }) => {
  const buttons: readonly (readonly [DeckEntry, boolean])[] = deck.flatMap(([id, primary]) => {
    const entry = entries.find((it) => it.id === id);
    return entry === undefined ? [] : [[entry, primary] as const];
  });
  const count = buttons.length + (extra === null ? 0 : 1);
  const leading = Math.trunc((buttons.length + 1) / 2);
  const button = ([entry, primary]: readonly [DeckEntry, boolean]) => (
    <LayoutWidget key={entry.id} widgetKey={deckKey(entry.id)} hideable={false} live={live} className="min-w-0 flex-1">
      <DeckButton entry={entry} primary={primary} buttons={count} className="w-full" />
    </LayoutWidget>
  );
  return (
    <div role="group" aria-label={FLIGHT_ACTIONS} className="flex items-center gap-[10px]">
      {buttons.slice(0, leading).map(button)}
      {center}
      {buttons.slice(leading).map(button)}
      {extra === null ? null : (
        <div className="min-w-0 flex-1" style={{ marginInlineStart: RAIL_STOP_GAP }}>
          <LayoutWidget widgetKey={deckKey(extra.id)} hideable={false} live={live}>
            <DeckButton entry={extra} primary={false} buttons={count} className="w-full" />
          </LayoutWidget>
        </div>
      )}
    </div>
  );
};

const OFFLINE_DECK = (): readonly DeckEntry[] => [
  DeckEntry({ id: 'takeoff', label: TAKE_OFF, icon: 'ic_flight_takeoff', enabled: false, onClick: () => undefined }),
  DeckEntry({ id: 'rtl', label: RETURN_HOME, icon: 'ic_home', enabled: false, onClick: () => undefined }),
];

const OfflineDeck = ({ className }: { className?: string }) => {
  const navigation = useContext(LocalAppNavigation);
  const flyScreen = useContext(LocalFlyScreenState);
  const status = offlineStatus(useQgcPath(OFFLINE_STATUS_VIEW));
  const onDevice = status?.onDevice === true;
  const saved = (status?.links.length ?? 0) > 0;
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
      <DeckButton
        entry={
          onDevice
            ? DeckEntry({ id: 'checkDevice', label: CHECK_DEVICE, icon: 'ic_developer_board', enabled: true, onClick: openDeviceTelemetry })
            : saved
              ? DeckEntry({ id: 'connect', label: 'Connect', icon: 'ic_link', enabled: true, onClick: () => (flyScreen.requestedSheet.value = OFFLINE_SHEET) })
              : DeckEntry({ id: 'addLink', label: 'Add a link', icon: 'ic_link', enabled: true, onClick: () => openAddLink(navigation) })
        }
        primary
        className="min-w-0 flex-1"
      />
    </div>
  );
};

export const LocalDeckTelemetry = createContext(true);

const DECISION_CARD_WIDTH = 460;
export const OSD_LABEL_ALPHA = 0.75;
export const STOP_MOTORS = 'Kill motors';
const FLIGHT_ACTIONS = 'Flight actions';
const RAIL_LIFT = 56;

const OSD_LABELS: ReadonlyMap<string, string> = new Map([
  ['distance to home', 'D'],
  ['altitude above home', 'H'],
  ['altitude', 'H'],
  ['ground speed', 'H.S'],
  ['climb rate', 'V.S'],
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

const PursuitLine = ({ pursuit, onAction }: { pursuit: AutoPursuit; onAction: (action: PursuitAction) => void }) => (
  <div className="flex w-full flex-wrap items-center gap-2">
    <Text typography="bodyMedium" color={pursuit.problem ? md('error') : md('onSurfaceVariant')} className="flex-1">
      {pursuit.headline}
    </Text>
    {pursuit.actions.map((action) => (
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
  const readiness = useGuidedReadiness(fly);
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

  return (
    <>
      {settings ? (
        <AircastSheet onDismissRequest={() => setSettings(false)} label="Flight mode settings">
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
          text={modeConfirmText(confirming)}
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
          leadingIcon={readiness !== null ? <Drawable name="ic_warning" style={{ color: aircast('warning') }} /> : <Drawable name="ic_check_circle" />}
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
  title = CHOOSE_READINGS,
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
  const [query, setQuery] = useState('');
  const connected = useHasVehicle();
  useLaunchedEffect([connected], async (scope) => {
    const catalogue = await Qgc.get(INSTRUMENT_GROUPS);
    const own = vehicleOwnGroup(catalogue);
    if (scope.isActive()) setGroups([...(own !== null ? [own] : []), ...instrumentGroups(catalogue)]);
  });
  const found = searchedGroups(shownFirst(groups, shownAtOpen), query);
  return (
    <AircastSheet onDismissRequest={onDismiss} label={title}>
      <div className="sticky top-0 z-10 bg-surface-container-low desktop:-top-dialog desktop:-mt-dialog desktop:pt-dialog">
        <Text typography="titleLarge" className="block px-4">{title}</Text>
        <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="block px-4 pb-2">{instrumentChoiceNote(chosen)}</Text>
        {groups.length === 0 ? null : <SearchPill value={query} onValueChange={setQuery} placeholder={SEARCH_READINGS} />}
      </div>
      {groups.length === 0 ? (
        <FootNote text={emptyCatalogueText(connected)} />
      ) : found.length === 0 ? (
        <FootNote text={noReadingFound(query)} />
      ) : (
        <div className="w-full overflow-auto">
          {found.map((group: InstrumentGroup) => (
            <Fragment key={`head${group.group}`}>
              <SectionHeader text={sentenceCase(group.title)} />
              {group.facts.map((fact: InstrumentFact) => (
                <ListItem
                  key={fact.path}
                  headlineContent={sentenceCase(fact.label)}
                  leadingContent={
                    <div className="flex items-center gap-4">
                      <Checkbox checked={chosen.includes(fact.path)} onCheckedChange={null} />
                      {fact.icon.trim() !== '' ? (
                        <span aria-hidden className="contents">
                          <ValueIcon name={fact.icon} tint={md('onSurfaceVariant')} size={20} />
                        </span>
                      ) : null}
                    </div>
                  }
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

const SURFACE = 'Surface';

export const returnLabel = (offer: GuidedOffer): string => (offer.title.trim() === '' ? RETURN_HOME : sentenceCase(offer.title));

export const returnIcon = (offer: GuidedOffer): DrawableName => (offer.title === SURFACE ? 'ic_upload' : 'ic_home');

export const altitudeLabel = (offer: GuidedOffer): string => sentenceCase(offer.title.replace(/^Change\s+/i, ''));

export const pauseClick = (pause: GuidedOffer, confirm: (action: GuidedAction) => void, openValue: (kind: GuidedValueKind) => void): void =>
  pause.carriesValue
    ? openValue(altitudeValue(true))
    : confirm(GuidedAction({ offerId: PAUSE, name: pause.title, confirm: promptOr(pause, 'Pause at current position'), destructive: false, answer: () => VehicleCommands.pauseRefusal() }));

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
                    confirm: promptOr(takeoff, 'Take off from the ground and hold position.'),
                    destructive: false,
                    answer: () => VehicleCommands.takeoffRefusal(),
                  }),
                )
              : openValue(takeoffValue(takeoff)),
        })
      : null,
    pause?.shown === true ? DeckEntry({ id: PAUSE, label: pause.title, icon: 'ic_pause', enabled: pause.ready, onClick: () => pauseClick(pause, confirm, openValue) }) : null,
    rtl?.shown === true
      ? DeckEntry({
          id: 'rtl',
          label: returnLabel(rtl),
          icon: returnIcon(rtl),
          enabled: rtl.ready,
          onClick: () =>
            confirm(
              GuidedAction({
                offerId: 'rtl',
                name: rtl.title,
                confirm: promptOr(rtl, 'Return to the launch position of the vehicle'),
                destructive: false,
                option: rtlOption === null ? null : ConfirmOption({ label: rtlOption, hint: rtl?.optionHint ?? '', run: (smart) => void VehicleCommands.returnToLaunchRefusal(smart).then(report) }),
                run: () => void VehicleCommands.returnToLaunchRefusal(false).then(report),
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
                answer: () => VehicleCommands.landRefusal(),
              }),
            ),
        })
      : null,
    speed?.shown === true ? DeckEntry({ id: 'changeSpeed', label: 'Speed', icon: 'ic_speed', enabled: speed.ready, onClick: () => openValue(speedValue(speed)) }) : null,
    altitude?.shown === true ? DeckEntry({ id: 'changeAltitude', label: altitudeLabel(altitude), icon: 'ic_height', enabled: altitude.ready, onClick: () => openValue(altitudeValue(false, altitude)) }) : null,
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

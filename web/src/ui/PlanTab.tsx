import { type ReactNode, useContext, useEffect, useMemo, useState } from 'react';
import { type Json, optBoolean, optInt, optJSONObject, optText } from '@/bridge/Json';
import { PlanCommands } from '@/bridge/BridgeCommands';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { AlertDialog, Button, DropdownMenu, DropdownMenuItem, HorizontalDivider, IconButton, OutlinedTextField, Surface, Text, TextButton } from '@/m3';
import { PlanMapScreen } from '@/map/PlanMap';
import type { PlanBar } from '@/map/PlanMapContent';
import type { PlanStat } from '@/map/PlanStrip';
import { sentenceCase } from '@/map/SentenceCase';
import type { TrackPoint } from '@/map/VehicleMap';
import { NEWEST_PATTERN, PlanFocus } from '@/map/VehicleSync';
import { type PlanAction, shortcutHint } from '@/map/PlanShortcuts';
import { Drawable, MaterialIcon } from '@/res/Icon';
import { aircast, md, withAlpha } from '@/theme/Colors';
import { LocalAppNavigation } from '@/ui/AppNavigation';
import { ItemEditor } from '@/ui/ItemEditor';
import { PlanDefaultsDialog } from '@/ui/PlanDefaults';
import {
  DEFAULT_PLAN_NAME,
  PlanConfirm,
  confirmCopy,
  planActions,
  planContainsItems,
  planIsDirty,
  planIsSyncing,
  planStatusText,
  planSyncProgress,
  planTitle,
  undrawnItemNames,
  undrawnItemsWarning,
} from '@/ui/PlanFileRules';
import { useRememberPlanFileActions, visualItems } from '@/ui/PlanFiles';
import { zoomedOutForTemplates } from '@/map/PlanBounds';
import { CREATE_FROM_TEMPLATE, NewPlanDialog, ZOOM_IN_FOR_TEMPLATE, planTemplates } from '@/ui/PlanTemplates';
import { usePlanRequests } from '@/ui/PlanRequests';
import { PlanTransformDialog } from '@/ui/PlanTransform';
import { RouteSettings } from '@/ui/RouteSettings';

const NOTICE_MILLIS = 4000;
const HEADER_ALPHA = 0.94;
const FLY_LABEL = 'Fly';
export const DOWNLOAD_LABEL = 'Download from vehicle';
const DISABLED_PILL_ALPHA = 0.38;
const PILL_HEIGHT = 40;
const HEADER_CORNER = 20;

export const APPLY_DEFAULT_ALTITUDE = 'core.plan.applyDefaultAltitude';
export const DISMISS_ALTITUDE_PROMPT = 'core.plan.dismissAltitudePrompt';

export type AltitudePrompt = { title: string; text: string };

export const LOAD_VEHICLE_PLAN = 'core.plan.loadVehiclePlan';
export const KEEP_CURRENT_PLAN = 'core.plan.keepCurrentPlan';

export type VehicleChangePrompt = { connected: boolean; dirty: boolean; aircraftItems: number | null };

export const vehicleChangePrompt = (view: Json | null): VehicleChangePrompt | null => {
  const prompt = optJSONObject(view, 'vehicleChangePrompt');
  if (prompt === null) return null;
  const count = optInt(prompt, 'aircraftItems', 0);
  return { connected: optBoolean(prompt, 'connected'), dirty: optBoolean(prompt, 'dirty'), aircraftItems: count > 0 ? count : null };
};

export type PromptCopy = { title: string; text: string; primary: string; secondary: string; primaryKeeps: boolean };

const itemsWord = (count: number): string => (count === 1 ? '1 item' : `${count} items`);

export const promptCopy = (prompt: VehicleChangePrompt): PromptCopy => {
  const stored = prompt.aircraftItems === null ? '' : ` (${itemsWord(prompt.aircraftItems)})`;
  if (prompt.connected && prompt.dirty) {
    return { title: 'Aircraft connected', text: `Keep the route you drew, or replace it with the route stored on the aircraft${stored}?`, primary: 'Keep my route', secondary: "Load the aircraft's route", primaryKeeps: true };
  }
  if (prompt.connected) {
    return { title: 'Aircraft connected', text: `Load the route stored on the aircraft${stored}, or keep this one?`, primary: "Load the aircraft's route", secondary: 'Keep this route', primaryKeeps: false };
  }
  if (prompt.dirty) {
    return { title: 'Aircraft disconnected', text: 'Keep the route you were working on?', primary: 'Keep my route', secondary: 'Discard it', primaryKeeps: true };
  }
  return { title: 'Aircraft disconnected', text: 'Keep this route, or start a new plan?', primary: 'Keep this route', secondary: 'Start a new plan', primaryKeeps: true };
};

export const applyAltitudePrompt = (view: Json | null): AltitudePrompt | null => {
  const prompt = optJSONObject(view, 'applyAltitudePrompt');
  return prompt === null ? null : { title: optText(prompt, 'title'), text: optText(prompt, 'text') };
};

export const planStatsLine = (stats: readonly PlanStat[]): string =>
  stats
    .flatMap((stat) =>
      stat.label === 'Items' ? (stat.value === '0' ? [] : [`${stat.value} ${stat.value === '1' ? 'item' : 'items'}`]) : stat.label === 'Max alt' ? [`max ${stat.value}`] : [stat.value],
    )
    .join(' · ');

export const headerLine = (status: string, syncing: boolean, stats: readonly PlanStat[]): string =>
  syncing || !stats.some((it) => it.label === 'Items' && it.value !== '0') ? status : planStatsLine(stats);

const PlanActionPill = ({
  label,
  icon,
  enabled,
  container,
  content,
  progress,
  onClick,
}: {
  label: string;
  icon: ReactNode;
  enabled: boolean;
  container: string;
  content: string;
  progress: number | null;
  onClick: () => void;
}) => (
  <Surface onClick={onClick} enabled={enabled} shape="full" color={container} contentColor={content} className="relative shrink-0 overflow-hidden" style={{ opacity: enabled ? 1 : DISABLED_PILL_ALPHA }}>
    {progress !== null ? <span className="absolute inset-y-0 left-0" style={{ width: `${progress * 100}%`, background: withAlpha(content, 0.28) }} /> : null}
    <span className="relative flex items-center gap-2 pr-6 pl-5" style={{ height: PILL_HEIGHT }}>
      {icon}
      <Text typography="labelLarge">{label}</Text>
    </span>
  </Surface>
);

export const SaveAsDialog = ({ initial, onDismiss, onSave }: { initial: string; onDismiss: () => void; onSave: (name: string) => void }) => {
  const [name, setName] = useState(initial);
  const named = name.trim() !== '';
  const confirm = () => {
    if (named) onSave(name);
  };
  return (
    <AlertDialog
      onDismissRequest={onDismiss}
      title="Save plan as"
      text={<OutlinedTextField value={name} onValueChange={setName} label="File name" singleLine autoFocus onDone={confirm} />}
      confirmButton={
        <TextButton enabled={named} onClick={confirm}>
          Save
        </TextButton>
      }
      dismissButton={<TextButton onClick={onDismiss}>Cancel</TextButton>}
    />
  );
};

export const PlanTab = ({ className, onFly = () => undefined }: { className?: string; onFly?: () => void }) => {
  const [notice, setNotice] = useState<string | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [pending, setPending] = useState<PlanConfirm | null>(null);
  const files = useRememberPlanFileActions((it) => setNotice(it));

  const planStatus = useQgcPath('view.plan');
  const containsItems = useMemo(() => planContainsItems(planStatus), [planStatus]);
  const dirty = useMemo(() => planIsDirty(planStatus), [planStatus]);
  const syncing = useMemo(() => planIsSyncing(planStatus), [planStatus]);
  const syncProgress = useMemo(() => planSyncProgress(planStatus), [planStatus]);
  const [showDefaults, setShowDefaults] = useState(false);
  const [showTransform, setShowTransform] = useState(false);
  const can = planActions(planStatus);
  const [undrawn, setUndrawn] = useState<readonly string[]>([]);
  const [centre, setCentre] = useState<readonly [number, number] | null>(null);
  const [zoom, setZoom] = useState<number | null>(null);
  const [newPlanOpen, setNewPlanOpen] = useState(false);
  const [startFrom, setStartFrom] = useState<string | null>(null);
  const documentName = files.documentName();
  const navigation = useContext(LocalAppNavigation);

  const startPlan = (template: string | null) => {
    if (template === null) {
      files.newPlan();
      return;
    }
    void (async () => {
      if (centre === null) {
        setNotice('The map has not reported its center yet.');
        return;
      }
      if (zoomedOutForTemplates(zoom)) {
        setNotice(ZOOM_IN_FOR_TEMPLATE);
        return;
      }
      const [lat, lon] = centre;
      const refused = await Qgc.refusalOf(CREATE_FROM_TEMPLATE, template, lat, lon);
      if (refused === null) PlanFocus.newPattern.value = NEWEST_PATTERN;
      else setNotice(refused);
    })();
  };

  useEffect(() => {
    void PlanCommands.setUndoTracking(true);
    return () => void PlanCommands.setUndoTracking(false);
  }, []);

  useLaunchedEffect([syncing, containsItems, documentName], async (scope) => {
    const names = undrawnItemNames(await visualItems());
    if (scope.isActive()) setUndrawn(names);
  });

  useLaunchedEffect([notice], async (scope) => {
    if (notice === null) return;
    await delay(NOTICE_MILLIS);
    if (scope.isActive()) setNotice(null);
  });

  const requestDownload = () => (dirty ? setPending(PlanConfirm.Download) : files.download());
  const requestOpen = () => (dirty ? setPending(PlanConfirm.Open) : files.open());
  const save = () => void files.save();
  const [savingAs, setSavingAs] = useState(false);
  const saveAs = () => setSavingAs(true);
  usePlanRequests(
    navigation.planRequest,
    planStatus !== null,
    { new: can.newPlan ? () => setNewPlanOpen(true) : null, open: can.open ? requestOpen : null, save: can.save ? save : null, saveAs: can.save ? saveAs : null },
    setNotice,
  );
  const hint = (action: PlanAction) => (
    <Text typography="labelMedium" color={md('onSurfaceVariant')}>
      {shortcutHint(action)}
    </Text>
  );

  const changePrompt = vehicleChangePrompt(planStatus);
  const altitudePrompt = applyAltitudePrompt(planStatus);
  const keep = () => void Qgc.invoke(KEEP_CURRENT_PLAN);
  const load = () => void Qgc.invoke(LOAD_VEHICLE_PLAN);
  const dismissAltitude = () => void Qgc.invoke(DISMISS_ALTITUDE_PROMPT);
  const patternOptions = files.patternChoice.options();
  const warning = undrawnItemsWarning(undrawn);

  const confirmDialog = (() => {
    if (pending === null) return null;
    const copy = confirmCopy(pending);
    const savedFirst = (then: () => void) => () =>
      void (async () => {
        if (await files.save()) then();
      })();
    const actions: Record<PlanConfirm['name'], readonly [() => void, (() => void) | null]> = {
      Open: [files.open, files.saveAndOpen],
      NewPlan: [() => startPlan(startFrom), savedFirst(() => startPlan(startFrom))],
      ClearMission: [files.clearMission, null],
      Download: [files.download, savedFirst(files.download)],
    };
    const [act, saveFirst] = actions[pending.name];
    return (
      <AlertDialog
        onDismissRequest={() => setPending(null)}
        title={copy.title}
        text={copy.body}
        confirmButton={
          <div className="flex flex-wrap items-center justify-end gap-2">
            <TextButton
              contentColor={copy.destructive ? md('error') : undefined}
              onClick={() => {
                setPending(null);
                act();
              }}
            >
              {copy.confirm}
            </TextButton>
            {copy.saveFirst === null || saveFirst === null ? null : (
              <Button
                onClick={() => {
                  setPending(null);
                  saveFirst();
                }}
              >
                {copy.saveFirst}
              </Button>
            )}
          </div>
        }
        dismissButton={<TextButton onClick={() => setPending(null)}>Keep editing</TextButton>}
      />
    );
  })();

  const header = (bar: PlanBar) => {
    const title = planTitle(documentName);
    const warned = notice === null && !syncing && bar.warning !== null;
    const line = notice ?? (warned ? bar.warning : null) ?? (!syncing ? bar.note : null) ?? headerLine(planStatusText(planStatus), syncing, bar.stats);
    const shownLine = line.trim() !== '' && line !== title ? line : null;
    return (
      <div className="flex flex-col gap-2 p-2">
        <div className="flex w-full items-center gap-2">
          <div className="min-w-0 flex-1">
            <Surface shape={HEADER_CORNER} color={withAlpha(md('surfaceContainer'), HEADER_ALPHA)}>
              <div className="flex min-h-10 flex-col justify-center px-4 py-1.5">
                <Text typography="titleSmall" maxLines={1} ellipsis>
                  {title}
                </Text>
                {shownLine !== null ? (
                  <Text typography="labelSmall" color={warned ? md('error') : md('onSurfaceVariant')} maxLines={notice === null ? 1 : 3} ellipsis>
                    {shownLine}
                  </Text>
                ) : null}
              </div>
            </Surface>
          </div>
          {bar.upload.shown ? (
            <PlanActionPill
              label={bar.upload.done ? FLY_LABEL : bar.upload.label}
              icon={<Drawable name={bar.upload.done ? 'ic_flight' : bar.upload.downloading ? 'ic_download' : bar.warning !== null ? 'ic_warning' : 'ic_upload'} size={20} />}
              enabled={bar.upload.enabled}
              container={bar.upload.done ? aircast('success') : bar.upload.emphasised ? md('primary') : md('surfaceContainerHighest')}
              content={bar.upload.done ? aircast('onSuccess') : bar.upload.emphasised ? md('onPrimary') : md('onSurface')}
              progress={syncing ? syncProgress : null}
              onClick={bar.upload.done ? onFly : bar.upload.onClick}
            />
          ) : containsItems ? (
            <PlanActionPill
              label={dirty ? 'Save' : 'Saved'}
              icon={dirty ? <MaterialIcon name="save" size={20} /> : <Drawable name="ic_check_circle" size={20} />}
              enabled={can.save && dirty}
              container={dirty ? md('primary') : md('surfaceContainerHighest')}
              content={dirty ? md('onPrimary') : md('onSurface')}
              progress={null}
              onClick={save}
            />
          ) : null}
          <Surface shape="full" color={withAlpha(md('surfaceContainer'), HEADER_ALPHA)} className="shrink-0">
            <IconButton onClick={() => setMenuOpen(true)} label="Plan menu">
              <Drawable name="ic_more_vert" />
            </IconButton>
            <DropdownMenu expanded={menuOpen} onDismissRequest={() => setMenuOpen(false)}>
              <DropdownMenuItem
                text="Undo"
                leadingIcon={<Drawable name="plan_undo" />}
                trailingIcon={hint('undo')}
                enabled={bar.undo !== null}
                onClick={() => {
                  setMenuOpen(false);
                  bar.undo?.();
                }}
              />
              <DropdownMenuItem
                text="Redo"
                leadingIcon={<Drawable name="ic_redo" />}
                trailingIcon={hint('redo')}
                enabled={bar.redo !== null}
                onClick={() => {
                  setMenuOpen(false);
                  bar.redo?.();
                }}
              />
              <HorizontalDivider />
              <DropdownMenuItem
                text="New plan…"
                leadingIcon={<Drawable name="ic_add" />}
                enabled={can.newPlan}
                onClick={() => {
                  setMenuOpen(false);
                  setNewPlanOpen(true);
                }}
              />
              <DropdownMenuItem
                text="Open plan…"
                leadingIcon={<Drawable name="ic_description" />}
                enabled={can.open}
                onClick={() => {
                  setMenuOpen(false);
                  requestOpen();
                }}
              />
              <DropdownMenuItem
                text="Save"
                leadingIcon={<MaterialIcon name="save" />}
                trailingIcon={hint('save')}
                enabled={can.save}
                onClick={() => {
                  setMenuOpen(false);
                  save();
                }}
              />
              <DropdownMenuItem
                text="Save as…"
                leadingIcon={<Drawable name="ic_edit" />}
                enabled={can.save}
                onClick={() => {
                  setMenuOpen(false);
                  saveAs();
                }}
              />
              <HorizontalDivider />
              <DropdownMenuItem
                text={DOWNLOAD_LABEL}
                leadingIcon={<Drawable name="ic_download" />}
                enabled={can.download}
                onClick={() => {
                  setMenuOpen(false);
                  requestDownload();
                }}
              />
              <DropdownMenuItem
                text="Clear mission"
                leadingIcon={<Drawable name="ic_delete" style={{ color: md('error') }} />}
                enabled={can.clearFromVehicle}
                color={md('error')}
                onClick={() => {
                  setMenuOpen(false);
                  setPending(PlanConfirm.ClearMission);
                }}
              />
              <HorizontalDivider />
              <DropdownMenuItem
                text="Import boundary…"
                leadingIcon={<Drawable name="ic_map" />}
                enabled={can.open}
                onClick={() => {
                  setMenuOpen(false);
                  files.importBoundary();
                }}
              />
              <DropdownMenuItem
                text="Export KML…"
                leadingIcon={<Drawable name="ic_send" />}
                enabled={can.exportKml}
                onClick={() => {
                  setMenuOpen(false);
                  files.exportKml();
                }}
              />
              <DropdownMenuItem
                text="Transform…"
                leadingIcon={<Drawable name="ic_straighten" />}
                enabled={containsItems}
                onClick={() => {
                  setMenuOpen(false);
                  setShowTransform(true);
                }}
              />
              <DropdownMenuItem
                text="Defaults…"
                leadingIcon={<Drawable name="ic_tune" />}
                onClick={() => {
                  setMenuOpen(false);
                  setShowDefaults(true);
                }}
              />
              <div aria-hidden className="pointer-events-none sticky bottom-0 -mt-6 h-6" style={{ background: `linear-gradient(transparent, ${md('surfaceContainer')})` }} />
            </DropdownMenu>
          </Surface>
        </div>
        {warning !== null ? (
          <Surface shape={HEADER_CORNER} color={md('errorContainer')}>
            <Text typography="bodySmall" color={md('onErrorContainer')} className="block px-4 py-2">
              {warning}
            </Text>
          </Surface>
        ) : null}
      </div>
    );
  };

  return (
    <>
      {savingAs ? (
        <SaveAsDialog
          initial={planTitle(documentName ?? DEFAULT_PLAN_NAME)}
          onDismiss={() => setSavingAs(false)}
          onSave={(name) => {
            setSavingAs(false);
            void files.saveAs(name);
          }}
        />
      ) : null}
      {newPlanOpen ? (
        <NewPlanDialog
          state={planTemplates(planStatus)}
          replacing={containsItems}
          zoomedOut={zoomedOutForTemplates(zoom)}
          onDismiss={() => setNewPlanOpen(false)}
          onPick={(template) => {
            setNewPlanOpen(false);
            setStartFrom(template);
            if (containsItems) setPending(PlanConfirm.NewPlan);
            else startPlan(template);
          }}
        />
      ) : null}
      {showDefaults ? <PlanDefaultsDialog view={planStatus} onDismiss={() => setShowDefaults(false)} /> : null}
      {changePrompt !== null
        ? (() => {
            const copy = promptCopy(changePrompt);
            return (
              <AlertDialog
                onDismissRequest={() => undefined}
                title={copy.title}
                text={copy.text}
                confirmButton={<Button onClick={copy.primaryKeeps ? keep : load}>{copy.primary}</Button>}
                dismissButton={<TextButton onClick={copy.primaryKeeps ? load : keep}>{copy.secondary}</TextButton>}
              />
            );
          })()
        : null}
      {altitudePrompt !== null ? (
        <AlertDialog
          onDismissRequest={dismissAltitude}
          title={sentenceCase(altitudePrompt.title)}
          text={altitudePrompt.text}
          confirmButton={<TextButton onClick={() => void Qgc.invoke(APPLY_DEFAULT_ALTITUDE)}>Yes</TextButton>}
          dismissButton={<TextButton onClick={dismissAltitude}>No</TextButton>}
        />
      ) : null}
      {showTransform ? <PlanTransformDialog onDismiss={() => setShowTransform(false)} /> : null}
      {confirmDialog}
      {patternOptions.length > 0 ? (
        <AlertDialog
          onDismissRequest={files.patternChoice.cancel}
          title="Import as which pattern?"
          text={
            <div className="flex flex-col">
              {patternOptions.map((name) => (
                <TextButton key={name} className="self-start" onClick={() => files.patternChoice.pick(name)}>
                  {name}
                </TextButton>
              ))}
            </div>
          }
          dismissButton={<TextButton onClick={files.patternChoice.cancel}>Cancel</TextButton>}
        />
      ) : null}
      <div className={`relative size-full ${className ?? ''}`}>
        <PlanMapScreen
          className="size-full"
          fitKey={files.opened()}
          onCentre={(lat: number, lon: number, level: number) => {
            setCentre([lat, lon]);
            setZoom(level);
          }}
          itemPanel={(index: number, at: TrackPoint | null, leg: string | null) => <ItemEditor index={index} at={at} mapCentre={centre} legDetail={leg} />}
          routeSettings={() => <RouteSettings plan={planStatus} />}
          onTemplates={() => setNewPlanOpen(true)}
          onDownload={requestDownload}
          onSave={can.save ? save : null}
          header={header}
        />
      </div>
    </>
  );
};

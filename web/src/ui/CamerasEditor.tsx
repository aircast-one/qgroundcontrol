import { useMemo, useState } from 'react';
import { CameraCommands, VideoCommands } from '@/bridge/BridgeCommands';
import { useQgcPath } from '@/bridge/QgcState';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { delay, same } from '@/kotlin/Std';
import { Button, HorizontalDivider, IconButton, OutlinedTextField, Text, TextButton } from '@/m3';
import { toDoubleOrNull } from '@/map/AltitudeEntry';
import { AircastSheet } from '@/map/AircastSheet';
import { Drawable, MaterialIcon } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { CAMERAS_VIEW, type CameraEntry, type CameraKind, addableKinds, addedName, cameraDetail, camerasReading, kindLabel, kindTitle } from '@/ui/Cameras';
import { CameraStatusDot } from '@/ui/CameraSwitch';
import { FootNote } from '@/ui/SetupRow';

const CAMERA_UNDO_WINDOW_MS = 6000;
const CAMERA_LIST_SETTLE_MS = 2000;
const ROW_PAD_HORIZONTAL = 16;
const ROW_PAD_VERTICAL = 10;
const ROW_MIN_HEIGHT = 64;
const ROW_GAP = 14;
const ROW_LINE_GAP = 2;
const SHEET_PAD = 24;
const SHEET_GAP = 12;
const GIMBAL_AIMED = 'Aimed by a gimbal';
const TILT_HELP = 'For a camera fixed to the frame: 0 looks straight ahead, -90 straight down. Leave empty when a gimbal aims it. Used to place home and the plan on the video.';
const TILT_UNREADABLE = 'Type the tilt as a number of degrees, like -30.';

export type CameraDraft = {
  stored: number | null;
  title: string;
  name: string;
  source: string;
  url: string;
  tilt: string;
  needsUrl: boolean;
  choosingKind: boolean;
  refusal: string | null;
};

export const CameraDraft = (fields: { stored: number | null; title: string; name: string; source: string; url: string; tilt?: string; needsUrl?: boolean; choosingKind?: boolean; refusal?: string | null }): CameraDraft => ({
  tilt: '',
  needsUrl: true,
  choosingKind: false,
  refusal: null,
  ...fields,
});

type RemovedCamera = { slot: number; name: string; source: string; url: string; tilt: number | null; title: string; active: boolean };

const undoRemoval = async (gone: RemovedCamera, storedCount: number): Promise<string | null> => {
  const added = await CameraCommands.add(gone.name, gone.source, gone.url, gone.tilt);
  const restored = added ?? (gone.slot < storedCount ? await CameraCommands.move(storedCount, gone.slot) : null);
  if (restored === null && gone.active) await VideoCommands.setActiveSource(gone.slot);
  return restored;
};

export const NEW_CAMERA_DRAFT: CameraDraft = CameraDraft({ stored: null, title: '', name: '', source: '', url: '', choosingKind: true });

export const editDraft = (camera: CameraEntry, kinds: readonly CameraKind[]): CameraDraft | null =>
  camera.stored === null
    ? null
    : CameraDraft({
        stored: camera.stored,
        title: camera.title,
        name: camera.name,
        source: camera.source,
        url: camera.url,
        tilt: tiltText(camera.tilt),
        needsUrl: !kinds.some((it) => it.raw === camera.source && !it.needsUrl),
      });

export const tiltText = (tilt: number | null): string => (tilt === null ? '' : tilt % 1.0 === 0.0 ? String(Math.trunc(tilt)) : String(tilt));

export const typedTilt = (text: string): number | null => toDoubleOrNull(text.trim().replaceAll(',', '.'));

export const tiltReadable = (text: string): boolean => text.trim() === '' || typedTilt(text) !== null;

export const pickedKind = (draft: CameraDraft, kind: CameraKind): CameraDraft => ({
  ...draft,
  source: kind.raw,
  needsUrl: kind.needsUrl,
  url: kind.needsUrl ? (draft.url.trim() === '' ? kind.defaultAddress : draft.url) : '',
  choosingKind: false,
  refusal: null,
});

export type CameraSave =
  | { kind: 'Add'; name: string; source: string; url: string; tilt: number | null }
  | { kind: 'Update'; slot: number; name: string; source: string; url: string; tilt: number | null };

export const CameraSave = {
  Add: (name: string, source: string, url: string, tilt: number | null): CameraSave => ({ kind: 'Add', name, source, url, tilt }),
  Update: (slot: number, name: string, source: string, url: string, tilt: number | null): CameraSave => ({ kind: 'Update', slot, name, source, url, tilt }),
};

export const cameraSave = (draft: CameraDraft): CameraSave => {
  const tilt = typedTilt(draft.tilt);
  return draft.stored !== null ? CameraSave.Update(draft.stored, draft.name, draft.source, draft.url, tilt) : CameraSave.Add(draft.name, draft.source, draft.url, tilt);
};

const written = (save: CameraSave): Promise<string | null> =>
  save.kind === 'Add' ? CameraCommands.add(save.name, save.source, save.url, save.tilt) : CameraCommands.update(save.slot, save.name, save.source, save.url, save.tilt);

export const refusedDraft = (current: CameraDraft | null, refusal: string | null): CameraDraft | null => (refusal === null || current === null ? null : { ...current, refusal });

export const CamerasEditor = ({ className }: { className?: string }) => {
  const view = useQgcPath(CAMERAS_VIEW);
  const reading = useMemo(() => camerasReading(view), [view]);
  const cameras = reading?.cameras ?? [];
  const kinds = reading?.kinds ?? [];
  const storedCount = reading?.stored.length ?? 0;
  const editable = reading?.readable === true;
  const [draft, setDraft] = useState<CameraDraft | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [removed, setRemoved] = useState<RemovedCamera | null>(null);
  const [pending, setPending] = useState(false);
  const [settlingFrom, setSettlingFrom] = useState<readonly unknown[] | null>(null);
  const storedNow = reading?.stored.map((it) => [it.name, it.source, it.url, it.tilt]) ?? null;
  const busy = pending || (settlingFrom !== null && same(settlingFrom, storedNow));
  const change = (action: () => Promise<string | null>, after: (refusal: string | null) => void) => {
    const before = storedNow;
    setPending(true);
    void (async () => {
      const refusal = await action();
      after(refusal);
      setSettlingFrom(refusal === null ? before : null);
      setPending(false);
    })();
  };

  useLaunchedEffect([settlingFrom], async (scope) => {
    if (settlingFrom !== null) {
      await delay(CAMERA_LIST_SETTLE_MS);
      if (scope.isActive()) setSettlingFrom(null);
    }
  });

  useLaunchedEffect([removed], async (scope) => {
    if (removed !== null) {
      await delay(CAMERA_UNDO_WINDOW_MS);
      if (scope.isActive()) setRemoved(null);
    }
  });

  const removeSlot = draft?.stored ?? null;
  const removeEntry = removeSlot === null ? null : (cameras.find((it) => it.stored === removeSlot) ?? null);
  const closed = (refusal: string | null) => setDraft((current) => refusedDraft(current, refusal));

  return (
    <>
      <div className={`flex flex-col overflow-y-auto ${className ?? ''}`}>
        <div className="flex w-full justify-end pe-1 pt-2">
          <IconButton onClick={() => setDraft(NEW_CAMERA_DRAFT)} enabled={editable && !busy} label="Add a video source">
            <Drawable name="ic_add" />
          </IconButton>
        </div>
        {reading !== null && !reading.readable ? <ErrorLine text={reading.reason} /> : null}
        {notice !== null ? <ErrorLine text={notice} /> : null}
        {reading !== null && cameras.length === 0 ? <FootNote text="No video sources yet. Cameras on the drone show up here by themselves. Tap + to add one." /> : null}
        {cameras.map((camera) => {
          const opened = editable ? editDraft(camera, kinds) : null;
          return (
            <div key={camera.slot}>
              <CameraRow camera={camera} onEdit={opened === null ? null : () => setDraft(opened)} />
              <HorizontalDivider />
            </div>
          );
        })}
        {removed !== null ? (
          <div className="flex w-full items-center py-1" style={{ paddingInline: ROW_PAD_HORIZONTAL }}>
            <Text typography="bodyMedium" className="flex-1">
              Removed {removed.title}.
            </Text>
            <TextButton
              enabled={!busy}
              onClick={() => {
                setRemoved(null);
                change(() => undoRemoval(removed, storedCount), setNotice);
              }}
            >
              Undo
            </TextButton>
          </div>
        ) : null}
      </div>
      {draft !== null ? (
        <CameraSheet
          draft={draft}
          kinds={addableKinds(reading, draft.stored !== null ? draft.source : null)}
          busy={busy}
          onChange={setDraft}
          onDismiss={() => setDraft(null)}
          onSave={() => change(() => written(cameraSave(draft)), closed)}
          onAddFixed={(kind) => change(() => CameraCommands.add(addedName(kind), kind.raw, '', null), closed)}
          onRemove={
            removeSlot === null || removeEntry === null
              ? null
              : () =>
                  change(
                    () => CameraCommands.remove(removeSlot),
                    (refusal) => {
                      setNotice(refusal);
                      if (refusal === null) setRemoved({ slot: removeSlot, name: removeEntry.name, source: removeEntry.source, url: removeEntry.url, tilt: removeEntry.tilt, title: removeEntry.title, active: removeEntry.active });
                      setDraft(null);
                    },
                  )
          }
        />
      ) : null}
    </>
  );
};

const ErrorLine = ({ text }: { text: string }) => (
  <Text typography="bodySmall" color={md('error')} className="block py-1" style={{ paddingInline: ROW_PAD_HORIZONTAL }}>
    {text}
  </Text>
);

const CameraRow = ({ camera, onEdit }: { camera: CameraEntry; onEdit: (() => void) | null }) => {
  const body = (
    <>
      <CameraStatusDot status={camera.status} />
      <span className="flex min-w-0 flex-1 flex-col" style={{ gap: ROW_LINE_GAP }}>
        <Text typography="bodyLarge" maxLines={1} ellipsis>
          {camera.title}
        </Text>
        <Text typography="bodyMedium" color={md('onSurfaceVariant')} maxLines={1} ellipsis>
          {cameraDetail(camera)}
        </Text>
        {camera.problem !== null ? (
          <Text typography="bodySmall" color={md('error')}>
            {camera.problem}
          </Text>
        ) : null}
      </span>
      {camera.active ? (
        <Text typography="labelMedium" color={md('onSurfaceVariant')}>
          On screen
        </Text>
      ) : null}
      {onEdit !== null ? <MaterialIcon name="keyboard_arrow_right" filled className="text-on-surface-variant rtl:-scale-x-100" /> : null}
    </>
  );
  const look = { minHeight: ROW_MIN_HEIGHT, paddingInline: ROW_PAD_HORIZONTAL, paddingBlock: ROW_PAD_VERTICAL, gap: ROW_GAP };
  return onEdit !== null ? (
    <button type="button" onClick={onEdit} className="flex w-full items-center text-left hover:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)]" style={look}>
      {body}
    </button>
  ) : (
    <div className="flex w-full items-center" style={look}>
      {body}
    </div>
  );
};

const CameraSheet = ({
  draft,
  kinds,
  busy,
  onChange,
  onDismiss,
  onSave,
  onAddFixed,
  onRemove,
}: {
  draft: CameraDraft;
  kinds: readonly CameraKind[];
  busy: boolean;
  onChange: (draft: CameraDraft) => void;
  onDismiss: () => void;
  onSave: () => void;
  onAddFixed: (kind: CameraKind) => void;
  onRemove: (() => void) | null;
}) => (
  <AircastSheet onDismissRequest={onDismiss}>
    <div className="flex flex-col overflow-y-auto" style={{ paddingInline: SHEET_PAD, paddingBottom: SHEET_PAD, gap: SHEET_GAP }}>
      {draft.choosingKind ? (
        <KindChooser
          title={draft.stored === null ? 'Add video source' : 'Type'}
          kinds={kinds}
          busy={busy}
          onPick={(kind) => (draft.stored === null && !kind.needsUrl ? onAddFixed(kind) : onChange(pickedKind(draft, kind)))}
          onCancel={draft.stored === null ? onDismiss : () => onChange({ ...draft, choosingKind: false })}
        />
      ) : (
        <CameraForm draft={draft} kind={kinds.find((it) => it.raw === draft.source) ?? null} busy={busy} onChange={onChange} onDismiss={onDismiss} onSave={onSave} onRemove={onRemove} />
      )}
    </div>
  </AircastSheet>
);

const groupedKinds = (kinds: readonly CameraKind[]): ReadonlyMap<string, readonly CameraKind[]> =>
  kinds.reduce((groups, kind) => new Map(groups).set(kind.group, [...(groups.get(kind.group) ?? []), kind]), new Map<string, readonly CameraKind[]>());

const KindChooser = ({ title, kinds, busy, onPick, onCancel }: { title: string; kinds: readonly CameraKind[]; busy: boolean; onPick: (kind: CameraKind) => void; onCancel: () => void }) => (
  <>
    <Text typography="titleMedium">{title}</Text>
    {[...groupedKinds(kinds)].map(([group, members]) => (
      <div key={group} className="flex flex-col">
        <Text typography="labelLarge" color={md('onSurfaceVariant')} className="block pt-2 pb-1">
          {group}
        </Text>
        {members.map((kind) => (
          <button key={kind.raw} type="button" disabled={busy} onClick={() => onPick(kind)} className="flex w-full flex-col py-2 text-left disabled:opacity-40" style={{ gap: ROW_LINE_GAP }}>
            <Text typography="bodyLarge">{kindTitle(kind)}</Text>
            {kind.description.trim() !== '' ? (
              <Text typography="bodySmall" color={md('onSurfaceVariant')}>
                {kind.description}
              </Text>
            ) : null}
          </button>
        ))}
      </div>
    ))}
    <div className="flex w-full justify-end">
      <TextButton onClick={onCancel}>Cancel</TextButton>
    </div>
  </>
);

const CameraForm = ({
  draft,
  kind,
  busy,
  onChange,
  onDismiss,
  onSave,
  onRemove,
}: {
  draft: CameraDraft;
  kind: CameraKind | null;
  busy: boolean;
  onChange: (draft: CameraDraft) => void;
  onDismiss: () => void;
  onSave: () => void;
  onRemove: (() => void) | null;
}) => {
  const [naming, setNaming] = useState(draft.stored !== null);
  const typeTitle = kind !== null ? kindTitle(kind) : kindLabel(draft.source);
  const canSave = !busy && (draft.url.trim() !== '' || !draft.needsUrl) && tiltReadable(draft.tilt);
  const save = () => {
    if (canSave) onSave();
  };
  const addressNote = draft.refusal ?? kind?.description ?? null;
  return (
    <>
      <Text typography="titleMedium">{draft.stored === null ? typeTitle : draft.title}</Text>
      <button type="button" disabled={busy} onClick={() => onChange({ ...draft, choosingKind: true })} className="flex w-full items-center py-2 text-left disabled:opacity-40">
        <span className="flex flex-1 flex-col" style={{ gap: ROW_LINE_GAP }}>
          <Text typography="labelMedium" color={md('onSurfaceVariant')}>
            Type
          </Text>
          <Text typography="bodyLarge">{typeTitle}</Text>
        </span>
        <MaterialIcon name="keyboard_arrow_right" filled label="Change type" className="text-on-surface-variant rtl:-scale-x-100" />
      </button>
      {naming ? (
        <OutlinedTextField value={draft.name} onValueChange={(name) => onChange({ ...draft, name, refusal: null })} label="Name" singleLine capitalization="words" onDone={save} className="w-full" />
      ) : (
        <div>
          <TextButton onClick={() => setNaming(true)}>Add a name</TextButton>
        </div>
      )}
      {draft.needsUrl ? (
        <OutlinedTextField
          value={draft.url}
          onValueChange={(typed) => onChange({ ...draft, url: typed, refusal: null })}
          label="Address"
          placeholder={kind?.hint ?? ''}
          singleLine
          isError={draft.refusal !== null}
          supportingText={addressNote !== null && addressNote.trim() !== '' ? addressNote : undefined}
          keyboardType="uri"
          autoCorrect={false}
          onDone={save}
          className="w-full"
        />
      ) : (
        <>
          {kind !== null && kind.description.trim() !== '' ? (
            <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
              {kind.description}
            </Text>
          ) : null}
          {draft.refusal !== null ? (
            <Text typography="bodySmall" color={md('error')}>
              {draft.refusal}
            </Text>
          ) : null}
        </>
      )}
      <OutlinedTextField
        value={draft.tilt}
        onValueChange={(typed) => onChange({ ...draft, tilt: typed, refusal: null })}
        label="Tilt (°)"
        placeholder={GIMBAL_AIMED}
        singleLine
        isError={!tiltReadable(draft.tilt)}
        supportingText={tiltReadable(draft.tilt) ? TILT_HELP : TILT_UNREADABLE}
        keyboardType="decimal"
        onDone={save}
        className="w-full"
      />
      <div className="flex items-center">
        {onRemove !== null ? (
          <TextButton onClick={onRemove} enabled={!busy}>
            <span style={{ color: md('error') }}>Remove</span>
          </TextButton>
        ) : null}
        <div className="flex-1" />
        <TextButton onClick={onDismiss}>Cancel</TextButton>
        <Button onClick={save} enabled={canSave}>
          Save
        </Button>
      </div>
    </>
  );
};

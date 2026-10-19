import { useContext, useEffect, useRef, useState } from 'react';
import { AssistChip, Button, FilledTonalButton, IconButton, ScrollableTabRow, Tab, Text, TextButton } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { CameraWizard } from '@/ui/CameraWizard';
import { type CameraCopy, type CameraView, type CamerasView, cameBack, camerasShown, liveness, parseCameras, portToAddress } from '@/ui/DeviceCameras';
import { type PortAddress } from '@/ui/DeviceCameraWizard';
import { DeviceBadgeText, DeviceChart, DeviceDisclosure, DeviceFact, DeviceNotice, DeviceSection, DeviceSwitchRow, LinkButton, QrDialog, levelColor, useCopy } from '@/ui/DeviceKit';
import { PreviewPlaceholder, WhepPreview } from '@/ui/DeviceKitCamera';
import type { DeviceScreenProps } from '@/ui/DeviceScreens';
import { devicePageIcon } from '@/ui/DeviceStatus';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { TargetRow } from '@/ui/DeviceConnectScreen';
import { deviceUrl, useDeviceView } from '@/ui/DeviceView';
import { FootNote, SetupRow, SetupState } from '@/ui/SetupRow';
import { LocalChangeNotice } from '@/ui/ValueRow';

const useComebacks = (view: CamerasView | null) => {
  const notice = useContext(LocalChangeNotice);
  const seen = useRef<ReadonlyMap<string, boolean>>(new Map());
  useEffect(() => {
    if (view === null) return;
    const back = cameBack(seen.current, view.cameras);
    if (back.length > 0) notice.show(`${back.join(', ')} ${back.length === 1 ? 'is' : 'are'} back — sending video.`, null);
    seen.current = liveness(view.cameras);
  }, [view]);
};

export const CamerasScreen = ({ query, onOpen }: DeviceScreenProps) => {
  const view = useDeviceView('cameras', parseCameras);
  const [picked, setPicked] = useState(query.get('camera') ?? '');
  const [adding, setAdding] = useState(query.has('add'));
  useComebacks(view);
  useEffect(() => {
    if (view?.cameras.length === 0) setAdding(true);
  }, [view]);
  if (view === null) return <FootNote text="Waiting for the device to answer…" />;
  const shown = camerasShown(view.cameras, picked, adding);
  const pick = (name: string) => {
    setAdding(false);
    setPicked(name);
  };
  return (
    <>
      {view.cameras.length > 0 ? (
        <ScrollableTabRow>
          {view.cameras.map((camera) => (
            <Tab
              key={camera.name}
              selected={shown.kind === 'camera' && shown.camera.name === camera.name}
              onClick={() => pick(camera.name)}
              text={
                <span className="flex items-center gap-2">
                  <span className="size-2 rounded-full" style={{ background: levelColor(camera.status.level) }} />
                  {camera.name}{' '}
                  <Text typography="labelSmall" color={levelColor(camera.status.level)}>
                    {camera.status.text}
                  </Text>
                </span>
              }
            />
          ))}
        </ScrollableTabRow>
      ) : null}
      {(!adding && view.cameras.length > 0) || view.cloudLink !== '' ? (
        <div className="flex flex-wrap items-center justify-between gap-2 px-list pt-2">
          {!adding && view.cameras.length > 0 ? <TextButton onClick={() => setAdding(true)}>+ Add camera</TextButton> : <span />}
          {view.cloudLink !== '' ? <LinkButton href={view.cloudLink} look="outlined" external>Watch in the cloud</LinkButton> : null}
        </div>
      ) : null}
      {shown.kind === 'camera' ? (
        <>
          <CameraDetail key={shown.camera.name} camera={shown.camera} scan={query.has('scan') && query.get('camera') === shown.camera.name} onOpen={onOpen} />
          <TopicRow page={shown.camera.topic} onOpen={onOpen} />
          <TopicRow page={view.video} onOpen={onOpen} />
        </>
      ) : shown.kind === 'wall' ? (
        <>
          <div className="grid gap-2 sm:grid-cols-2">
            {view.cameras.map((camera) => (
              <CameraTile key={camera.name} camera={camera} onPick={() => pick(camera.name)} />
            ))}
          </div>
          <TopicRow page={view.video} onOpen={onOpen} />
        </>
      ) : (
        <DeviceSection title="Add a camera">
          <CameraWizard view={view.add} onDone={pick} />
        </DeviceSection>
      )}
    </>
  );
};

const TopicRow = ({ page, onOpen }: { page: CamerasView['video']; onOpen: (href: string) => void }) => (
  <SetupRow title={page.title} subtitle={page.summary} icon={devicePageIcon(page.icon)} state={page.needs ? SetupState.NeedsAttention : SetupState.Neutral} onClick={() => onOpen(page.href)} />
);

const Picture = ({ camera }: { camera: CameraView }) =>
  camera.preview !== '' ? <WhepPreview endpoint={deviceUrl(camera.preview)} watch={deviceUrl(camera.watch)} name={camera.name} /> : <PreviewPlaceholder text={camera.placeholder} />;

const CameraTile = ({ camera, onPick }: { camera: CameraView; onPick: () => void }) => (
  <section aria-label={camera.name} className="flex flex-col gap-2 px-list py-3">
    <div className="flex flex-wrap items-center gap-3">
      <button type="button" onClick={onPick} className="text-left">
        <Text typography="titleMedium">{camera.name}</Text>
      </button>
      <DeviceBadgeText badge={camera.status} />
      <span className="flex-1" />
      <FilledTonalButton onClick={onPick}>Open</FilledTonalButton>
    </div>
    <Picture camera={camera} />
    {camera.diagnosis !== null ? (
      <Text typography="bodyMedium" color={levelColor(camera.diagnosis.level)}>
        {camera.diagnosis.text}
      </Text>
    ) : null}
    <div className="-mx-list">
      <DeviceSwitchRow
        title="Record to disk"
        checked={camera.recording}
        on={{ method: 'PUT', path: `/api/streams/${camera.name}/record` }}
        off={{ method: 'DELETE', path: `/api/streams/${camera.name}/record` }}
        success={camera.recording ? `Recording turned off for ${camera.name} — download it under Settings → Recordings.` : `Recording turned on for ${camera.name}.`}
      />
    </div>
  </section>
);

const CameraDetail = ({ camera, scan, onOpen }: { camera: CameraView; scan: boolean; onOpen: (href: string) => void }) => {
  const [wizard, setWizard] = useState<{ readonly scan: boolean; readonly fixPort: PortAddress | null } | null>(scan ? { scan: true, fixPort: null } : null);
  if (wizard !== null)
    return (
      <DeviceSection title={`Set up ${camera.name}`} badge={camera.status}>
        <CameraWizard view={camera.wizard} scan={wizard.scan} fixPort={wizard.fixPort} onCancel={() => setWizard(null)} onDone={() => setWizard(null)} />
      </DeviceSection>
    );
  const reconfigure = () => setWizard({ scan: !camera.live, fixPort: null });
  return (
    <DeviceSection title={camera.name} badge={camera.status} action={camera.live ? <TextButton onClick={reconfigure}>Edit</TextButton> : null}>
      {camera.shared ? (
        <>
          <div className="px-list">
            <DeviceBadgeText badge={{ level: DeviceLevel.Warn, text: 'shared source' }} />
          </div>
          <DeviceNotice
            notice={{
              level: DeviceLevel.Warn,
              text: 'Another camera pulls this same RTSP source. One camera fed to two pulls splits its video and audio across the streams — remove the duplicate camera for a stable feed.',
            }}
          />
        </>
      ) : null}
      <div className="px-list py-2">
        <div className="max-w-xl">
          <Picture camera={camera} />
        </div>
      </div>
      <div className="flex min-w-0 flex-col">
        {camera.live ? (
          <div className="flex flex-wrap items-center gap-2 px-list py-2" role="group" aria-label="Watch">
            <LinkButton href={deviceUrl(camera.watch)} look="filled" external>
              ▶ Watch live
            </LinkButton>
            {camera.copies.map((copy) => (
              <CopyButton key={copy.label} copy={copy} />
            ))}
          </div>
        ) : null}
        {camera.facts.map((fact) => (
          <DeviceFact key={fact.label} label={fact.label} value={fact.value} />
        ))}
        {camera.bitrate !== null ? <DeviceChart chart={camera.bitrate} /> : null}
        {camera.diagnosis !== null ? (
          <>
            <DeviceNotice notice={camera.diagnosis} />
            {camera.fix !== null ? (
              <div className="px-list">
                <Button onClick={() => setWizard({ scan: camera.fix?.scan ?? false, fixPort: portToAddress(camera.fix) })}>{camera.fix.label}</Button>
              </div>
            ) : null}
            <div className="px-list py-1">
              <Text typography="bodySmall" color={md('onSurfaceVariant')}>
                This page keeps checking — it updates by itself the moment the camera answers again.
              </Text>
            </div>
          </>
        ) : null}
        {!camera.live && camera.fix === null ? (
          <div className="px-list py-2">
            <Button onClick={reconfigure}>Reconfigure</Button>
          </div>
        ) : null}
      </div>
      {camera.links.length > 0 ? (
        camera.live ? (
          <section aria-label="Watch in another app">
            <div className="px-list pt-3">
              <Text typography="labelLarge" color={md('onSurfaceVariant')}>
                Watch in another app
              </Text>
            </div>
            <CameraLinks links={camera.links} />
          </section>
        ) : (
          <DeviceDisclosure title="Links — they will play once it's back">
            <CameraLinks links={camera.links} />
          </DeviceDisclosure>
        )
      ) : null}
      {camera.chips.length > 0 ? (
        <div className="flex flex-wrap gap-2 px-list py-3" role="group" aria-label="Where it goes">
          {camera.chips.map((chip) => (
            <AssistChip
              key={chip.key}
              onClick={() => onOpen(chip.href)}
              leadingIcon={<span className="size-2 rounded-full" style={{ background: levelColor(chip.level) }} />}
              label={chip.label}
              trailingIcon={<MaterialIcon name="arrow_forward" size={16} />}
            />
          ))}
        </div>
      ) : null}
    </DeviceSection>
  );
};

const CameraLinks = ({ links }: { links: CameraView['links'] }) => (
  <>
    {links.map((link) => (
      <TargetRow key={link.url} target={link} />
    ))}
  </>
);

const CopyButton = ({ copy }: { copy: CameraCopy }) => {
  const doCopy = useCopy();
  const [qr, setQr] = useState(false);
  return (
    <span className="inline-flex items-center">
      <FilledTonalButton onClick={() => doCopy(copy.text)}>
        <MaterialIcon name="content_copy" size={18} />
        {copy.label}
      </FilledTonalButton>
      {copy.qr ? (
        <IconButton label={`Show ${copy.label.replace(/^Copy /, '')} as a QR code`} onClick={() => setQr(true)}>
          <MaterialIcon name="qr_code_2" />
        </IconButton>
      ) : null}
      {qr ? <QrDialog title={copy.label.replace(/^Copy /, '')} text={copy.text} onDismiss={() => setQr(false)} /> : null}
    </span>
  );
};

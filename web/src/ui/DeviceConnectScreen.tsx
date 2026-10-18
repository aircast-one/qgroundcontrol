import { useEffect, useState } from 'react';
import { FilledTonalButton, Text } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';
import {
  type ConnectTarget,
  type ConnectView,
  GCS_KEY,
  type GcsChoice,
  REACH_RECHECK_MS,
  type Reach,
  chosenGcs,
  parseConnect,
  probeBlocked,
  probeReach,
  readStored,
  shownTargets,
  writeStored,
} from '@/ui/DeviceConnect';
import { CopyRow, DeviceDisclosure, DeviceNotice, DeviceSection, DeviceText } from '@/ui/DeviceKit';
import type { DeviceScreenProps } from '@/ui/DeviceScreens';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { useDeviceView } from '@/ui/DeviceView';
import { FootNote } from '@/ui/SetupRow';

const useReach = (url: string): Reach | null => {
  const [reach, setReach] = useState<Reach | null>(null);
  useEffect(() => {
    if (probeBlocked(url, location.protocol)) return undefined;
    const check = () => (document.hidden ? undefined : void probeReach(url).then(setReach));
    check();
    const timer = setInterval(check, REACH_RECHECK_MS);
    document.addEventListener('visibilitychange', check);
    return () => {
      clearInterval(timer);
      document.removeEventListener('visibilitychange', check);
    };
  }, [url]);
  return reach;
};

const ReachLine = ({ reach }: { reach: Reach }) => (
  <div className="px-list pb-2">
    <Text typography="labelMedium" color={reach === 'ok' ? aircast('success') : aircast('warning')}>
      {reach === 'ok' ? 'This address answers from here' : 'No answer from here — your ground station may not reach it either'}
    </Text>
  </div>
);

const TargetRow = ({ target }: { target: ConnectTarget }) => {
  const reach = useReach(target.reach);
  const label = target.tag === '' ? target.label : `${target.label} · ${target.tag}`;
  return (
    <div role="group" aria-label={target.label}>
      <CopyRow label={label} text={target.url} hint={target.hint} qr={target.qr} level={target.down ? DeviceLevel.Warn : DeviceLevel.Neutral} />
      {reach !== null ? <ReachLine reach={reach} /> : null}
      {target.alternatives.length > 0 ? (
        <div className="ml-list border-l-2 border-outline-variant">
          <DeviceText text="Doesn't resolve on Android or Windows? Use the address instead:" />
          {target.alternatives.map((alt) => (
            <CopyRow key={alt.url} label={alt.label} text={alt.url} />
          ))}
        </div>
      ) : null}
    </div>
  );
};

const TargetRows = ({ targets, choice }: { targets: readonly ConnectTarget[]; choice: GcsChoice | null }) => (
  <>
    {shownTargets(targets, choice).map((target) => (
      <TargetRow key={target.url} target={target} />
    ))}
  </>
);

const GcsChooser = ({ choices, choice, onPick }: { choices: readonly GcsChoice[]; choice: GcsChoice | null; onPick: (value: string) => void }) => (
  <div role="radiogroup" aria-label="Your ground station" className="flex flex-wrap gap-2 px-list py-2">
    {choices.map((it) => (
      <button
        key={it.value}
        type="button"
        role="radio"
        aria-checked={it.value === choice?.value}
        onClick={() => onPick(it.value)}
        className={`inline-flex h-8 items-center gap-2 rounded-shape-sm px-3 text-label-large ${it.value === choice?.value ? 'bg-secondary-container text-on-secondary-container' : 'border border-outline text-on-surface-variant hover:bg-[color-mix(in_srgb,currentColor_8%,transparent)]'}`}
      >
        {it.value === choice?.value ? <MaterialIcon name="check" size={18} /> : null}
        {it.label}
      </button>
    ))}
  </div>
);

const Subheading = ({ text }: { text: string }) => (
  <div className="px-list pt-4 pb-1">
    <Text typography="titleSmall">{text}</Text>
  </div>
);

export const ConnectScreen = ({ onOpen }: DeviceScreenProps) => {
  const view = useDeviceView('connect', parseConnect);
  const [saved, setSaved] = useState(() => readStored(GCS_KEY));
  if (view === null) return <FootNote text="Waiting for the device to answer…" />;
  const choice = chosenGcs(view.choices, saved);
  const pick = (value: string) => {
    writeStored(GCS_KEY, value);
    setSaved(value);
  };
  return view.noCamera !== null ? (
    <DeviceSection title="Cameras">
      <DeviceText text="There is no camera on this device yet, so there is nothing to point a ground station at." />
      <div className="px-list py-2">
        <FilledTonalButton onClick={() => view.noCamera !== null && onOpen(view.noCamera.href)}>{view.noCamera.label}</FilledTonalButton>
      </div>
    </DeviceSection>
  ) : (
    <ConnectLinks view={view} choice={choice} onPick={pick} onOpen={onOpen} />
  );
};

const ConnectLinks = ({ view, choice, onPick, onOpen }: { view: ConnectView; choice: GcsChoice | null; onPick: (value: string) => void; onOpen: (href: string) => void }) => (
  <>
    <DeviceSection title="Your ground station">
      <GcsChooser choices={view.choices} choice={choice} onPick={onPick} />
      <DeviceText text="QGroundControl and Mission Planner play the RTSP link; a browser, OBS or Aircast QGC play WebRTC. This browser remembers your answer." />
      <DeviceText text={view.scope} />
      <DeviceDisclosure title="How to connect a ground station">
        <DeviceText text="Each link below says where to paste it in your app; video and telemetry are two separate links." />
        <DeviceText text="Ground station on another device? Scan that link's QR code from there — no retyping." />
      </DeviceDisclosure>
    </DeviceSection>
    {view.cameras.map((camera) => (
      <DeviceSection key={camera.name} title={camera.name} badge={camera.status}>
        {camera.notice !== '' ? <DeviceNotice notice={{ level: DeviceLevel.Warn, text: camera.notice }} /> : null}
        {camera.fix !== null ? (
          <div className="px-list py-2">
            <FilledTonalButton onClick={() => camera.fix !== null && onOpen(camera.fix.href)}>{camera.fix.label}</FilledTonalButton>
          </div>
        ) : null}
        <TargetRows targets={camera.targets} choice={choice} />
      </DeviceSection>
    ))}
    {view.telemetry !== null ? (
      <DeviceSection title="Telemetry" badge={view.telemetry.status}>
        <DeviceText text="Video and telemetry are separate links — a ground station needs both." />
        <TargetRows targets={view.telemetry.targets} choice={choice} />
      </DeviceSection>
    ) : null}
    {view.trouble !== null ? (
      <DeviceSection title="Troubleshooting">
        <DeviceDisclosure title="This link didn't play in my app" summary="Reach the device by address, or fall back to a player that takes anything.">
          {view.trouble.fallback.length > 0 ? (
            <>
              <Subheading text="When nothing else will play" />
              <TargetRows targets={view.trouble.fallback} choice={choice} />
            </>
          ) : null}
          {view.trouble.addresses !== null ? (
            <>
              <Subheading text={view.trouble.addresses.title} />
              <DeviceText text="Windows without Bonjour, and some networks, cannot look up a .local name — and a tailnet name needs Tailscale running there too. These links use this device's address instead: pick the one on the same network as your ground station." />
              {view.trouble.addresses.groups.map((group) => (
                <div key={group.label}>
                  <div className="px-list pt-3">
                    <Text typography="labelMedium" color={md('onSurfaceVariant')}>
                      {group.label}
                    </Text>
                  </div>
                  <TargetRows targets={group.targets} choice={choice} />
                </div>
              ))}
            </>
          ) : null}
        </DeviceDisclosure>
      </DeviceSection>
    ) : null}
  </>
);

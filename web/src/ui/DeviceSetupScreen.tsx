import { useContext, useEffect, useRef, useState } from 'react';
import { Button, FilledTonalButton, OutlinedButton, Text, TextButton } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';
import { LocalAppNavigation } from '@/ui/AppNavigation';
import { CameraWizard } from '@/ui/CameraWizard';
import { ActionButton, DeviceBadgeText, DeviceNotice, DeviceText, LinkButton } from '@/ui/DeviceKit';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { TailscaleSection } from '@/ui/DeviceNetworkScreen';
import type { DeviceScreenProps } from '@/ui/DeviceScreens';
import { type SetupCamera, type SetupInternet, type SetupStep, type SetupTelemetry, type SetupView, cameraHoldsTheWizard, cameraStuck, clampStep, parseSetup, setupNav } from '@/ui/DeviceSetup';
import { type DevicePage, devicePageIcon } from '@/ui/DeviceStatus';
import { EndpointRows } from '@/ui/DeviceTelemetryScreen';
import { deviceUrl, useDeviceView } from '@/ui/DeviceView';
import { WifiJoin } from '@/ui/DeviceWifiWizard';
import { FootNote, SetupRow, SetupState } from '@/ui/SetupRow';

export const SetupScreen = ({ onOpen }: DeviceScreenProps) => {
  const view = useDeviceView('setup', parseSetup);
  if (view === null) return <FootNote text="Waiting for the device to answer…" />;
  return <SetupWizard view={view} onOpen={onOpen} />;
};

const SetupWizard = ({ view, onOpen }: { view: SetupView; onOpen: (href: string) => void }) => {
  const navigation = useContext(LocalAppNavigation);
  const [step, setStep] = useState(view.start);
  const [reconfiguring, setReconfiguring] = useState(false);
  const [wizardStep, setWizardStep] = useState(1);
  const top = useRef<HTMLDivElement>(null);
  useEffect(() => {
    top.current?.scrollIntoView({ block: 'nearest' });
  }, [step]);
  const total = view.steps.length;
  const current = view.steps[clampStep(step, total) - 1] ?? view.steps[0];
  if (current === undefined) return null;
  const wizardShown = view.camera.wizard.isNew || reconfiguring;
  const nav = setupNav(step, total, cameraHoldsTheWizard(current.key, wizardShown, wizardStep), cameraStuck(current.key, view.camera.problem));
  const go = (next: number) => setStep(clampStep(next, total));
  const close = () => {
    navigation.settingsOpen.value = false;
  };
  const reconfigure = () => {
    setWizardStep(1);
    setReconfiguring(true);
  };
  return (
    <>
      <div ref={top}>
        <StepTrail steps={view.steps} step={step} />
      </div>
      <section aria-label={current.title} className="flex flex-col">
        <div className="flex items-start gap-3 px-list pt-5 pb-1">
          <div role="heading" aria-level={2} className="min-w-0 flex-1">
            <Text typography="titleLarge">{current.title}</Text>
          </div>
          {current.status !== null ? <DeviceBadgeText badge={current.status} /> : null}
        </div>
        <DeviceText text={current.lead} />
        {current.key === 'network' ? <InternetStep internet={view.internet} onOpen={onOpen} /> : null}
        {current.key === 'tailscale' ? <TailscaleSection tailscale={view.remote} /> : null}
        {current.key === 'camera' ? (
          <CameraStep camera={view.camera} wizardShown={wizardShown} onReconfigure={reconfigure} onStop={() => setReconfiguring(false)} onWizardStep={setWizardStep} onOpen={onOpen} />
        ) : null}
        {current.key === 'telemetry' ? <TelemetryStep telemetry={view.telemetry} /> : null}
        {current.key === 'done'
          ? view.checks.map((check) => (
              <SetupRow
                key={check.label}
                title={check.label}
                status={check.status.text}
                state={check.status.level === DeviceLevel.Ready ? SetupState.Done : SetupState.Neutral}
                onClick={() => go(check.step)}
              />
            ))
          : null}
      </section>
      <div className="flex flex-wrap items-center justify-between gap-2 px-list pt-4 pb-6">
        {nav.back ? <OutlinedButton onClick={() => go(step - 1)}>← Back</OutlinedButton> : <span />}
        <div className="flex flex-wrap items-center justify-end gap-2">
          {nav.later ? <TextButton onClick={close}>Finish setup later</TextButton> : null}
          {nav.next === 'next' ? <Button onClick={() => go(step + 1)}>Next →</Button> : null}
          {nav.next === 'skip' ? <OutlinedButton onClick={() => go(step + 1)}>Skip this step →</OutlinedButton> : null}
          {nav.finish ? <ActionButton label="Go to the fly view" request={{ method: 'POST', path: '/api/setup/complete' }} onDone={close} /> : null}
        </div>
      </div>
    </>
  );
};

const StepTrail = ({ steps, step }: { steps: readonly SetupStep[]; step: number }) => (
  <ol aria-label="Setup steps" className="flex items-center gap-1.5 px-list pt-4">
    {steps.map((it, index) => {
      const here = index + 1 === step;
      const done = !here && it.status !== null;
      return (
        <li key={it.key} aria-current={here ? 'step' : undefined} className="flex min-w-0 items-center gap-1.5">
          {index > 0 ? <span aria-hidden className="h-px w-2 shrink-0 bg-outline-variant sm:w-4" /> : null}
          <span
            className={`grid size-6 shrink-0 place-items-center rounded-full border text-label-small ${here ? 'border-primary bg-primary text-on-primary' : done ? 'border-transparent' : 'border-outline-variant text-on-surface-variant'}`}
            style={done ? { background: aircast('success'), color: md('surface') } : undefined}
          >
            {done ? <MaterialIcon name="check" size={16} label="done" /> : index + 1}
          </span>
          <Text typography="labelMedium" color={here ? md('onSurface') : md('onSurfaceVariant')} className={here ? 'truncate' : 'truncate max-sm:hidden'}>
            {it.label}
          </Text>
        </li>
      );
    })}
  </ol>
);

const PageRow = ({ page, onOpen }: { page: DevicePage; onOpen: (href: string) => void }) => (
  <SetupRow title={page.title} subtitle={page.summary} icon={devicePageIcon(page.icon)} state={page.needs ? SetupState.NeedsAttention : SetupState.Neutral} onClick={() => onOpen(page.href)} />
);

const InternetStep = ({ internet, onOpen }: { internet: SetupInternet; onOpen: (href: string) => void }) => (
  <>
    <DeviceNotice notice={internet.online} />
    <WifiJoin reopen={internet.reopen} initiallyOpen={internet.joinOpen} />
    <PageRow page={internet.connection} onOpen={onOpen} />
    <DeviceText text={internet.hint} />
  </>
);

const CameraStep = ({
  camera,
  wizardShown,
  onReconfigure,
  onStop,
  onWizardStep,
  onOpen,
}: {
  camera: SetupCamera;
  wizardShown: boolean;
  onReconfigure: () => void;
  onStop: () => void;
  onWizardStep: (step: number) => void;
  onOpen: (href: string) => void;
}) => {
  const Reconfigure = camera.watch !== '' ? FilledTonalButton : Button;
  return (
    <>
      {camera.problem !== '' ? <DeviceNotice notice={{ level: DeviceLevel.Warn, text: camera.problem }} /> : null}
      {wizardShown ? (
        <CameraWizard key={camera.wizard.isNew ? '' : camera.wizard.name} view={camera.wizard} onCancel={camera.wizard.isNew ? null : onStop} onStep={onWizardStep} onDone={onStop} />
      ) : (
        <>
          <div className="flex flex-wrap items-center gap-2 px-list py-2" role="group" aria-label={camera.wizard.name}>
            {camera.watch !== '' ? (
              <LinkButton href={deviceUrl(camera.watch)} look="filled" external>
                ▶ Watch live
              </LinkButton>
            ) : null}
            <Reconfigure onClick={onReconfigure}>Reconfigure</Reconfigure>
          </div>
          <PageRow page={camera.cameras} onOpen={onOpen} />
          <DeviceText text={camera.hint} />
        </>
      )}
    </>
  );
};

const TelemetryStep = ({ telemetry }: { telemetry: SetupTelemetry }) => (
  <>
    {telemetry.waiting !== null ? <DeviceNotice notice={telemetry.waiting} /> : null}
    <EndpointRows endpoints={telemetry.endpoints} />
    <DeviceText text={telemetry.hint} />
  </>
);

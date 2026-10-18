import { useState } from 'react';
import { FilledTonalButton, OutlinedTextField, Text } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { ActionButton, CopyRow, DeviceDisclosure, DeviceNotice, DeviceSection, DeviceText, RefusalText, levelColor, useDeviceAction } from '@/ui/DeviceKit';
import { ButtonRow, LinkButton, StackedFact, StatusLine, TypedConfirmDialog } from '@/ui/DeviceKitSettings';
import type { DeviceScreenProps } from '@/ui/DeviceScreens';
import { CloudStage, type CloudAccount, type DeviceSettingsView, type DeviceStat, SERVICES, type ServiceRow, type SoftwareNotice, type SoftwareStatus, parseDeviceSettings } from '@/ui/DeviceSettings';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { deviceUrl, useDeviceView } from '@/ui/DeviceView';
import { FootNote } from '@/ui/SetupRow';

export const DeviceSettingsScreen = ({ onOpen }: DeviceScreenProps) => {
  const view = useDeviceView('device', parseDeviceSettings);
  if (view === null) return <FootNote text="Waiting for the device to answer…" />;
  return (
    <>
      <HealthLine view={view} onOpen={onOpen} />
      {view.notice !== null ? <DeviceNotice notice={view.notice} /> : null}
      <DeviceSection title="Status">
        {view.stats.map((stat) => (stat.label === SERVICES ? <ServicesStat key={stat.label} stat={stat} services={view.services} /> : <StatRow key={stat.label} stat={stat} />))}
      </DeviceSection>
      <CloudSection cloud={view.cloud} />
      <SoftwareSection software={view.software} />
      <DeviceSection title="More devices">
        <ProvisionSteps />
      </DeviceSection>
      <ResetSection hostname={view.hostname} />
    </>
  );
};

const HealthLine = ({ view, onOpen }: { view: DeviceSettingsView; onOpen: (href: string) => void }) => (
  <div className="flex items-center gap-3 px-list pt-4 pb-1">
    <span className="size-2.5 shrink-0 rounded-full" style={{ background: levelColor(view.health.level) }} />
    <Text typography="titleMedium" color={levelColor(view.health.level)} className="min-w-0 flex-1">
      {view.health.text}
    </Text>
    {view.health.href !== '' ? <FilledTonalButton onClick={() => onOpen(view.health.href)}>Check</FilledTonalButton> : null}
  </div>
);

const StatText = ({ stat }: { stat: DeviceStat }) => (
  <span className="flex min-w-0 flex-1 flex-col">
    <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
      {stat.label}
    </Text>
    <Text typography="titleMedium" color={stat.level === DeviceLevel.Neutral ? md('onSurface') : levelColor(stat.level)} className="break-words">
      {stat.value}
    </Text>
    <Text typography="bodySmall" color={md('onSurfaceVariant')} className="break-all">
      {stat.hint}
    </Text>
  </span>
);

const StatRow = ({ stat }: { stat: DeviceStat }) => (
  <div className="flex px-list py-2" role="group" aria-label={stat.label}>
    <StatText stat={stat} />
  </div>
);

const ServicesStat = ({ stat, services }: { stat: DeviceStat; services: readonly ServiceRow[] }) => {
  const [open, setOpen] = useState(false);
  return (
    <div role="group" aria-label={stat.label}>
      <button type="button" aria-expanded={open} onClick={() => setOpen(!open)} className="flex w-full items-center gap-4 px-list py-2 text-left hover:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)]">
        <StatText stat={stat} />
        <Text typography="labelMedium" color={md('primary')}>
          {open ? 'Hide' : 'Show details'}
        </Text>
        <MaterialIcon name={open ? 'expand_less' : 'expand_more'} style={{ color: md('onSurfaceVariant') }} />
      </button>
      {open ? (
        <div className="flex flex-col pb-2">
          <ul aria-label="Supervised units" className="flex flex-col">
            {services.map((service) => (
              <li key={service.label} className="flex items-center justify-between gap-3 px-list py-1.5">
                <Text typography="bodyMedium">{service.label}</Text>
                <Text typography="labelLarge" color={levelColor(service.level)}>
                  {service.state}
                </Text>
              </li>
            ))}
          </ul>
          <StatusLine text="Supervised by systemd — a unit that stops is restarted automatically." />
        </div>
      ) : null}
    </div>
  );
};

const CloudSection = ({ cloud }: { cloud: CloudAccount }) => (
  <DeviceSection title="Cloud account" badge={cloud.status}>
    {cloud.stage === CloudStage.Linked ? (
      <>
        <DeviceText text="This device is linked to your Aircast cloud account. The ID below only matters if support asks for it." />
        <CopyRow label="Device ID (for support)" text={cloud.deviceId} />
        {cloud.watchUrl !== '' ? (
          <ButtonRow>
            <LinkButton href={cloud.watchUrl} external tonal={false}>
              Watch in the cloud
            </LinkButton>
          </ButtonRow>
        ) : null}
      </>
    ) : cloud.stage === CloudStage.SigningIn ? (
      <>
        <DeviceText text="Open the sign-in page, log in (or sign up) with Google, and authorize this device. It connects automatically once you approve." />
        <ButtonRow>
          <LinkButton href={cloud.signInUrl} external>
            Open sign-in page
          </LinkButton>
        </ButtonRow>
        <CopyRow label="Or go to the link and enter this code" text={cloud.userCode} />
      </>
    ) : (
      <CloudConnect key={cloud.issuer} cloud={cloud} />
    )}
  </DeviceSection>
);

const CloudConnect = ({ cloud }: { cloud: CloudAccount }) => {
  const action = useDeviceAction();
  const [issuer, setIssuer] = useState(cloud.issuer);
  const refusedIssuer = action.refusal?.field === 'issuer';
  return (
    <>
      <DeviceText text="Not connected. Linking signs this device in to an Aircast cloud account; it is optional — every local and self-hosted path works without it." />
      {cloud.error !== '' ? <DeviceNotice notice={{ level: DeviceLevel.Warn, text: cloud.error }} /> : null}
      <DeviceDisclosure title="Advanced · environment" summary="Which Aircast cloud to sign in to" initiallyOpen={refusedIssuer}>
        <div className="flex flex-col gap-2 px-list py-2">
          <OutlinedTextField
            label="Cloud API"
            ariaLabel="Cloud API"
            value={issuer}
            placeholder="https://api.aircast.one"
            keyboardType="uri"
            autoCorrect={false}
            isError={refusedIssuer}
            supportingText="Leave this alone unless you were told to use a different Aircast environment."
            onValueChange={setIssuer}
          />
        </div>
      </DeviceDisclosure>
      <RefusalText refusal={action.refusal} field={action.refusal?.field ?? ''} />
      <ButtonRow>
        <FilledTonalButton enabled={!action.busy} onClick={() => void action.run({ method: 'POST', path: '/api/cloud/connect', body: { issuer } }, 'Sign-in started — the code appears here in a moment.')}>
          Connect
        </FilledTonalButton>
      </ButtonRow>
    </>
  );
};

const SoftwareNoticeRow = ({ notice }: { notice: SoftwareNotice }) => (
  <>
    <DeviceNotice notice={{ level: notice.level, text: notice.text === '' ? notice.title : `${notice.title} ${notice.text}` }} />
    {notice.detail !== '' ? (
      <DeviceDisclosure title={notice.detailLabel}>
        <pre className="mx-list mb-2 max-h-48 overflow-auto rounded-shape-sm bg-surface-container-high p-3 text-body-small break-words whitespace-pre-wrap">{notice.detail}</pre>
      </DeviceDisclosure>
    ) : null}
  </>
);

const SoftwareSection = ({ software }: { software: SoftwareStatus }) => (
  <DeviceSection title="Software" badge={software.status}>
    <StackedFact label="Running" value={software.summary} />
    {software.notices.map((notice) => (
      <SoftwareNoticeRow key={notice.title} notice={notice} />
    ))}
    {software.candidate !== null ? (
      <div className="mx-list my-2 flex flex-col gap-2 rounded-shape-md bg-surface-container-high p-4" role="group" aria-label="Available update">
        <Text typography="titleMedium">{software.candidate.title}</Text>
        <div className="flex flex-wrap items-center gap-2">
          <ActionButton
            label={software.candidate.install}
            request={{ method: 'POST', path: '/api/update' }}
            success={software.candidate.success}
            confirm={{ title: 'Install the update?', text: software.candidate.confirm, confirm: 'Install' }}
          />
          <LinkButton href={software.candidate.notesUrl} external tonal={false}>
            What's new
          </LinkButton>
        </div>
      </div>
    ) : null}
    {software.notes.map((note) => (
      <DeviceText key={note} text={note} />
    ))}
    <div className="flex flex-wrap items-center justify-between gap-2 px-list py-2">
      <Text typography="bodySmall" color={md('onSurfaceVariant')}>
        {software.checked}
      </Text>
      {software.canCheck ? <ActionButton label="Check now" request={{ method: 'POST', path: '/api/update/check' }} success="Checked for updates." tonal={false} /> : null}
    </div>
  </DeviceSection>
);

const PROVISION_STEPS = [
  "Export this device's settings.",
  'Add your tailnet auth key to the file. It is not exported, because this page answers anyone who can reach the device.',
  "Copy the file onto the new card's boot partition as aircast-provision.json.",
  'Boot it. It joins, picks up the camera and the data caps, then deletes the file so the key does not stay on the card.',
];

const ProvisionSteps = () => (
  <DeviceDisclosure title="Set up another device like this one" summary="Export these settings for the next card.">
    <ol className="ml-list list-decimal pr-list pl-5">
      {PROVISION_STEPS.map((step) => (
        <li key={step} className="py-1">
          <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
            {step}
          </Text>
        </li>
      ))}
    </ol>
    <DeviceText text="The file carries the camera's own address, sign-in included — that is what makes it worth copying. Treat it like the password it contains." />
    <ButtonRow>
      <LinkButton href={deviceUrl('/api/provision/export')} download>
        Export these settings
      </LinkButton>
    </ButtonRow>
    <StatusLine text="Airframes two through five are this one with a different serial." />
  </DeviceDisclosure>
);

const ResetSection = ({ hostname }: { hostname: string }) => {
  const action = useDeviceAction();
  const [asking, setAsking] = useState(false);
  return (
    <DeviceSection title="Reset device">
      <DeviceText text="Reboots and reverts the device to factory settings — camera, telemetry and all Wi-Fi and added network settings are wiped, and the setup wizard runs again. Tailscale stays connected so the device remains reachable." />
      <ButtonRow>
        <FilledTonalButton enabled={!action.busy} containerColor={md('errorContainer')} contentColor={md('onErrorContainer')} onClick={() => setAsking(true)}>
          Reset to defaults
        </FilledTonalButton>
      </ButtonRow>
      <RefusalText refusal={action.refusal} field={action.refusal?.field ?? ''} />
      {asking ? (
        <TypedConfirmDialog
          title="Reset to factory settings?"
          text="It reboots and wipes the camera, telemetry and all Wi-Fi and added network settings, then runs setup again. Tailscale is preserved."
          label={`Type ${hostname} to confirm`}
          expected={hostname}
          confirm="Reset"
          onConfirm={(typed) => void action.run({ method: 'POST', path: '/api/setup/reset', body: { confirm: typed } }, 'Resetting — the device is rebooting to factory settings; reconnect in a minute.')}
          onDismiss={() => setAsking(false)}
        />
      ) : null}
    </DeviceSection>
  );
};

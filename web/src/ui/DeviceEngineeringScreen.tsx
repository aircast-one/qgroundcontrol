import { useState } from 'react';
import { FilledTonalButton, OutlinedTextField, RadioButton, Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';
import { useDeviceDraft } from '@/ui/DeviceDraft';
import { ActionButton, ConfirmDialog, DeviceColumns, DeviceDisclosure, DeviceNotice, DeviceSection, DeviceSwitchRow, DeviceText, LinkButton, RefusalText, SecretField, useDeviceAction } from '@/ui/DeviceKit';
import { ButtonRow, StatusLine } from '@/ui/DeviceKitSettings';
import { type DiagnosticsSettings, type EngineeringView, type PublishSignIn, modeNamed, parseEngineering, shownFields, signInBody, signInValues } from '@/ui/DeviceEngineering';
import type { DeviceScreenProps } from '@/ui/DeviceScreens';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { deviceUrl, useDeviceView } from '@/ui/DeviceView';
import { FootNote } from '@/ui/SetupRow';

export const EngineeringScreen = ({ onOpen }: DeviceScreenProps) => {
  const view = useDeviceView('engineering', parseEngineering);
  if (view === null) return <FootNote text="Waiting for the device to answer…" />;
  return (
    <DeviceColumns
      left={
        <>
          <BackendSection view={view} />
          <PublishSection publish={view.publish} onOpen={onOpen} />
        </>
      }
      right={
        <>
          <DeviceSection title="Device logs">
            <DeviceText text="Everything every service on this device has logged, gzipped." />
            <ButtonRow>
              <LinkButton href={deviceUrl('/api/logs')} download>
                Download logs
              </LinkButton>
            </ButtonRow>
          </DeviceSection>
          <DiagnosticsSection diagnostics={view.diagnostics} />
        </>
      }
    />
  );
};

const BackendSection = ({ view }: { view: EngineeringView }) => (
  <DeviceSection title="What carries video" badge={view.backend.status}>
    <StatusLine text="aircastd's own engine, or mediamtx as a separate service — switching restarts the device." />
    <DeviceText text={view.backend.text} />
    <ButtonRow>
      <ActionButton
        label={view.backend.label}
        request={{ method: 'PUT', path: '/api/stream/media-backend', body: { backend: view.backend.switchTo } }}
        success={view.backend.success}
        confirm={{ title: 'Restart the device?', text: view.backend.confirm, confirm: 'Switch' }}
      />
    </ButtonRow>
    <DeviceDisclosure title="Technical details">
      <DeviceText text="Both backends bind the same ports — 8554, 8888, 8889, 1935, 8890 — so only one can run, and the choice is read once at startup. Switching writes the setting and restarts aircastd into it; if the restart fails the setting still stands and the next boot lands on it. Camera paths carry across in both directions. Only the engine can send a camera to a wfb-ng packet radio." />
    </DeviceDisclosure>
  </DeviceSection>
);

const PublishSection = ({ publish, onOpen }: { publish: PublishSignIn; onOpen: (href: string) => void }) => (
  <DeviceSection title="Publishing sign-in" badge={publish.status}>
    <DeviceText text="Devices on the Aircast cloud sign in automatically once connected to a cloud account. Custom servers are set up under Advanced; publishing to a specific server — LiveKit, Cloudflare — is configured per camera on the Cameras page." />
    {publish.awaitingCloud ? (
      <>
        <ButtonRow>
          <FilledTonalButton onClick={() => onOpen('/settings/device')}>Connect the cloud account</FilledTonalButton>
        </ButtonRow>
        <StatusLine text="Nothing to fill in here — the sign-in arrives with the cloud account." />
      </>
    ) : null}
    <ButtonRow>
      <TextButton onClick={() => onOpen('/streaming')}>Choose where cameras publish</TextButton>
    </ButtonRow>
    <DeviceDisclosure title="Advanced · sign-in credentials" summary="How this device signs in to video servers.">
      <SignInEditor publish={publish} />
    </DeviceDisclosure>
  </DeviceSection>
);

const SignInEditor = ({ publish }: { publish: PublishSignIn }) => {
  const action = useDeviceAction();
  const chosen = useDeviceDraft({ mode: publish.mode });
  const edits = useDeviceDraft(signInValues(publish));
  const [asking, setAsking] = useState(false);
  const mode = chosen.values.mode ?? publish.mode;
  const setMode = (value: string) => chosen.edit('mode', value);
  const picked = modeNamed(publish, mode);
  const dirty = mode !== publish.mode;
  const confirm = dirty ? (picked?.confirm ?? '') : '';
  const save = () => {
    const body = signInBody(publish, mode, edits.values);
    void action.run({ method: 'PUT', path: '/api/stream/sfu/oauth', body }, 'Publishing sign-in saved.').then((done) => {
      if (!done) return;
      chosen.accept({ mode });
      edits.accept(body);
    });
  };
  return (
    <div className="flex flex-col gap-3 px-list py-2">
      <div role="radiogroup" aria-label="Sign-in method" className="flex flex-col gap-2">
        {publish.modes.map((option) => (
          <div key={option.value} className="flex cursor-pointer items-start gap-3 rounded-shape-md border border-outline-variant p-3" onClick={() => setMode(option.value)}>
            <RadioButton selected={option.value === mode} label={option.title} onClick={() => setMode(option.value)} />
            <span className="flex min-w-0 flex-1 flex-col">
              <Text typography="bodyLarge">{option.title}</Text>
              <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
                {option.desc}
              </Text>
            </span>
          </div>
        ))}
      </div>
      {mode === 'aircast' && publish.environment !== '' ? (
        <div className="flex flex-col">
          <Text typography="bodySmall" color={md('onSurfaceVariant')}>
            Environment
          </Text>
          <Text typography="bodyMedium" monospace className="break-all">
            {publish.environment}
          </Text>
        </div>
      ) : null}
      {shownFields(publish, mode).map((field) =>
        field.secret ? (
          <SecretField
            key={field.name}
            label={field.label}
            value={edits.values[field.name] ?? field.value}
            placeholder={field.placeholder}
            isError={action.refusal?.field === field.name}
            supportingText={action.refusal?.field === field.name ? action.refusal.message : undefined}
            onValueChange={(value) => edits.edit(field.name, value)}
          />
        ) : (
          <OutlinedTextField
            key={field.name}
            label={field.label}
            ariaLabel={field.label}
            value={edits.values[field.name] ?? field.value}
            placeholder={field.placeholder}
            keyboardType="uri"
            autoCorrect={false}
            isError={action.refusal?.field === field.name}
            supportingText={action.refusal?.field === field.name ? action.refusal.message : undefined}
            onValueChange={(value) => edits.edit(field.name, value)}
          />
        ),
      )}
      {dirty ? (
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          {picked?.unsaved ?? ''}
        </Text>
      ) : publish.verdict.level === DeviceLevel.Blocked ? (
        <DeviceNotice notice={publish.verdict} />
      ) : (
        <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
          {publish.verdict.text}
        </Text>
      )}
      {action.refusal !== null && !shownFields(publish, mode).some((field) => field.name === action.refusal?.field) ? <RefusalText refusal={action.refusal} field={action.refusal.field} /> : null}
      <div className="flex flex-wrap items-center justify-between gap-2">
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          Stored on the device only.
        </Text>
        <FilledTonalButton enabled={!action.busy} onClick={() => (confirm === '' ? save() : setAsking(true))}>
          {picked?.submit ?? 'Save'}
        </FilledTonalButton>
      </div>
      {asking ? <ConfirmDialog title="Clear the sign-in?" text={confirm} confirm="Clear" onConfirm={save} onDismiss={() => setAsking(false)} /> : null}
    </div>
  );
};

const DiagnosticsSection = ({ diagnostics }: { diagnostics: DiagnosticsSettings }) => (
  <DeviceSection title="Remote diagnostics" badge={diagnostics.status}>
    <DeviceText text={diagnostics.hint} />
    <DiagnosticsForm diagnostics={diagnostics} />
    <DeviceSwitchRow
      title="Stream this device's logs"
      checked={diagnostics.streaming}
      enabled={diagnostics.canStream}
      on={{ method: 'PUT', path: '/api/diagnostics', body: { enabled: 'true' } }}
      off={{ method: 'PUT', path: '/api/diagnostics', body: { enabled: 'false' } }}
      success="Streaming logs."
      offSuccess="Stopped streaming logs."
      confirmOn={{ title: 'Stream the logs?', text: diagnostics.confirm, confirm: 'Stream' }}
    />
    <StatusLine text={diagnostics.state} />
    <StatusLine text="Ships this device's logs to an endpoint you name, so support can read them without the device in front of them." />
  </DeviceSection>
);

const DiagnosticsForm = ({ diagnostics }: { diagnostics: DiagnosticsSettings }) => {
  const action = useDeviceAction();
  const draft = useDeviceDraft({ endpoint: diagnostics.endpoint, token: diagnostics.token });
  const endpoint = draft.values.endpoint ?? diagnostics.endpoint;
  const token = draft.values.token ?? diagnostics.token;
  const refused = (field: string) => action.refusal?.field === field;
  const save = () => void action.run({ method: 'POST', path: '/api/diagnostics/config', body: { endpoint, token } }, 'Diagnostics endpoint saved.').then((done) => (done ? draft.accept({ endpoint, token }) : undefined));
  return (
    <div className="flex flex-col gap-3 px-list py-2" role="group" aria-label="Diagnostics endpoint">
      <OutlinedTextField
        label="OTLP endpoint (host:port)"
        ariaLabel="OTLP endpoint (host:port)"
        value={endpoint}
        placeholder="otel.example.com:443"
        keyboardType="uri"
        autoCorrect={false}
        isError={refused('endpoint')}
        supportingText={refused('endpoint') ? action.refusal?.message : undefined}
        onValueChange={(value) => draft.edit('endpoint', value)}
      />
      <SecretField
        label={diagnostics.tokenLabel}
        value={token}
        placeholder="none"
        isError={refused('token')}
        supportingText={refused('token') ? action.refusal?.message : undefined}
        onValueChange={(value) => draft.edit('token', value)}
      />
      {action.refusal !== null && !refused('endpoint') && !refused('token') ? <RefusalText refusal={action.refusal} field={action.refusal.field} /> : null}
      <div className="flex flex-wrap items-center justify-between gap-2">
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          Stored on the device only.
        </Text>
        <FilledTonalButton enabled={!action.busy} onClick={save}>
          Save
        </FilledTonalButton>
      </div>
    </div>
  );
};

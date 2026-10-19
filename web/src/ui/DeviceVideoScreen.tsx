import { useContext, useState } from 'react';
import { Checkbox, FilledTonalButton, OutlinedTextField, Text } from '@/m3';
import { md } from '@/theme/Colors';
import { ActionButton, CopyRow, DeviceBadgeText, DeviceColumns, DeviceDisclosure, DeviceForm, DeviceNotice, DeviceSection, DeviceText, LinkButton, RefusalText } from '@/ui/DeviceKit';
import { ButtonRow, StackedFact, StatusLine, StepText } from '@/ui/DeviceKitSettings';
import type { DeviceScreenProps } from '@/ui/DeviceScreens';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { SHARING_OFF, SharingStage, type DetectModel, type DetectionSettings, type PublicSharing, type RadioSettings, modelPath, parseVideo, uploadModel } from '@/ui/DeviceVideo';
import { type DeviceRefusal, useDeviceView } from '@/ui/DeviceView';
import { FootNote, SetupRow } from '@/ui/SetupRow';
import { LocalChangeNotice } from '@/ui/ValueRow';

export const VideoScreen = ({ onOpen }: DeviceScreenProps) => {
  const view = useDeviceView('video', parseVideo);
  if (view === null) return <FootNote text="Waiting for the device to answer…" />;
  const sharing = (
    <>
      <SharingSection sharing={view.sharing} />
      <StatusLine text="These apply to every camera on this device. Which camera goes where is set on the camera itself." />
    </>
  );
  const detection = (
    <>
      <DetectionSection detection={view.detection} />
      <StatusLine text="Detection runs on the device and sends boxes as data — the picture itself is untouched. Pick the model for what the camera is looking at: an aerial model hallucinates trucks on a desk. Draw-a-box tracking works whichever model is active." />
    </>
  );
  return (
    <>
      {view.radio !== null ? (
        <DeviceColumns
          left={<RadioSection radio={view.radio} />}
          right={
            <>
              {sharing}
              {detection}
            </>
          }
        />
      ) : (
        <DeviceColumns left={sharing} right={detection} />
      )}
      <SetupRow title={view.engineering.title} subtitle={view.engineering.summary} icon="ic_terminal" onClick={() => onOpen(view.engineering.href)} />
    </>
  );
};

const RadioSection = ({ radio }: { radio: RadioSettings }) => (
  <DeviceSection title="Packet radio" badge={radio.status}>
    <DeviceText text="The channel has to match the ground station exactly — a mismatch looks the same as a dead radio. Saving restarts the radio, so the link drops for a few seconds." />
    <DeviceForm
      key={`${radio.channel}|${radio.region}|${radio.txPower}`}
      method="PUT"
      path="/api/stream/wfb/radio"
      submit="Save"
      success="Radio settings applied."
      note="Restarts the radio."
      confirm={{ title: 'Restart the radio?', text: 'Change the radio and restart it? The link drops for a few seconds — do this on the ground.', confirm: 'Restart' }}
      fields={[
        { name: 'channel', label: 'Channel', value: radio.channel, keyboard: 'number', hint: '1-14 on 2.4 GHz, 32-177 on 5 GHz. 165 is 5825 MHz.' },
        { name: 'region', label: 'Region', value: radio.region, hint: 'Two-letter country code for the regulatory domain. BO permits the highest transmit power.' },
        { name: 'tx_power', label: 'Transmit power', value: radio.txPower, keyboard: 'number', placeholder: 'driver default', hint: '0-30. Leave empty for the driver default — the legal ceiling and the heat the adapter can shed are both airframe-specific.' },
      ]}
    />
    <DeviceDisclosure title="Technical details">
      <DeviceText text={radio.details} />
    </DeviceDisclosure>
  </DeviceSection>
);

const SharingSection = ({ sharing }: { sharing: PublicSharing }) => (
  <DeviceSection title="Public viewer (Cloudflare)" badge={sharing.status}>
    {sharing.stage === SharingStage.Live ? (
      <>
        <DeviceText text="Public sharing is on. Every camera that publishes to Cloudflare shows a public share link — hand it to anyone, no login or VPN, and it survives the backend going down." />
        <CopyRow label="Bridge Worker" text={sharing.baseUrl} />
        <ButtonRow>
          <ActionButton
            label="Turn off"
            request={{ method: 'PUT', path: '/api/stream/cfbridge', body: SHARING_OFF }}
            success="Public sharing turned off."
            confirm={{ title: 'Turn off public sharing?', text: 'Turn off public sharing? Existing share links stop working.', confirm: 'Turn off' }}
          />
        </ButtonRow>
      </>
    ) : sharing.stage === SharingStage.Deploy ? (
      <>
        <StepText step={1} title="Deploy the bridge to your Cloudflare account" text="One click — Cloudflare copies the bridge into your account and creates its storage. Sign in when asked." />
        <ButtonRow>
          <LinkButton href={sharing.deployUrl} external>
            Deploy to Cloudflare
          </LinkButton>
        </ButtonRow>
        <StepText step={2} title="Paste these two keys when Cloudflare asks for secrets" />
        <CopyRow label="SIGN_KEY" text={sharing.signKey} hint="Signs share links (generated on this device)." />
        <CopyRow label="DEVICE_TOKEN" text={sharing.deviceToken} hint="Lets this device report the live session (generated on this device)." />
        <StatusLine text="Your Cloudflare app secret is sent automatically by this device — you don't need to set it." />
        <StepText step={3} title="Paste the Worker URL Cloudflare gives you" />
        <DeviceForm
          method="PUT"
          path="/api/stream/cfbridge"
          submit="Connect"
          success="Public sharing is on."
          extra={{ signKey: sharing.signKey, deviceToken: sharing.deviceToken }}
          fields={[{ name: 'baseUrl', label: 'Worker URL', value: sharing.baseUrl, placeholder: 'https://aircast-cf-bridge.<you>.workers.dev', keyboard: 'uri' }]}
        />
        <ButtonRow>
          <ActionButton
            label="Start over"
            tonal={false}
            request={{ method: 'PUT', path: '/api/stream/cfbridge', body: SHARING_OFF }}
            success="Setup cleared."
            confirm={{ title: 'Start over?', text: 'Discard the generated keys and start over?', confirm: 'Discard' }}
          />
        </ButtonRow>
      </>
    ) : (
      <>
        <DeviceText text="Give a camera that publishes to Cloudflare Realtime a public share link — anyone opens it in a browser, no login and no VPN, and it keeps working even if the cloud backend is down. You host the bridge in your own Cloudflare account (free tier is plenty)." />
        <ButtonRow>
          <ActionButton label="Set up public sharing" request={{ method: 'POST', path: '/api/stream/cfbridge/keys' }} success="Keys generated — follow the deploy steps." />
        </ButtonRow>
      </>
    )}
  </DeviceSection>
);

const DetectionSection = ({ detection }: { detection: DetectionSettings }) => (
  <DeviceSection title="Detection model">
    <StackedFact label="In use" value={detection.status.text} />
    {detection.unavailable ? (
      <DeviceNotice notice={{ level: DeviceLevel.Warn, text: "Detection is unavailable — this device has no AI accelerator, so the models below can't run. Draw-a-box tracking still works on every camera." }} />
    ) : null}
    {detection.models.length === 0 ? <DeviceText text="No models installed yet." /> : detection.models.map((model) => <ModelRow key={model.name} model={model} />)}
    <DeviceDisclosure title="Add a model" summary="A .hef compiled for Hailo-8, with its class labels.">
      <ModelUploadForm />
    </DeviceDisclosure>
  </DeviceSection>
);

const ModelRow = ({ model }: { model: DetectModel }) => (
  <div className="flex flex-wrap items-center gap-x-3 gap-y-1 px-list py-2" role="group" aria-label={model.name}>
    <div className="flex min-w-0 flex-1 basis-48 flex-col">
      <Text typography="bodyLarge" className="break-all">
        {model.name}
      </Text>
      {model.about !== '' ? (
        <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
          {model.about}
        </Text>
      ) : null}
      <span className="flex items-center gap-3">
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          {model.size}
        </Text>
        <DeviceBadgeText badge={model.classes} />
      </span>
    </div>
    {model.active ? (
      <DeviceBadgeText badge={{ level: DeviceLevel.Ready, text: 'Active' }} />
    ) : (
      <span className="flex items-center gap-1">
        <ActionButton label="Use" request={{ method: 'PUT', path: '/api/stream/detect/settings', body: { hef: model.path } }} success="Model switched — detection restarts on the new model." />
        <ActionButton
          label="Delete"
          tonal={false}
          request={{ method: 'DELETE', path: modelPath(model.name) }}
          success="Model deleted."
          confirm={{ title: `Delete ${model.name}?`, text: `Delete ${model.name}? Anyone relying on it loses it from this device.`, confirm: 'Delete' }}
        />
      </span>
    )}
  </div>
);

const fileInput =
  'block w-full max-w-md cursor-pointer rounded-shape-xs border border-outline text-body-small text-on-surface-variant file:mr-3 file:cursor-pointer file:border-0 file:bg-secondary-container file:px-3 file:py-2 file:text-label-large file:text-on-secondary-container';

const ModelUploadForm = () => {
  const notice = useContext(LocalChangeNotice);
  const [hef, setHef] = useState<File | null>(null);
  const [labels, setLabels] = useState<File | null>(null);
  const [about, setAbout] = useState('');
  const [use, setUse] = useState(true);
  const [busy, setBusy] = useState(false);
  const [refusal, setRefusal] = useState<DeviceRefusal | null>(null);
  const upload = () => {
    if (hef === null) return setRefusal({ message: 'Choose a .hef model file.', field: '' });
    setBusy(true);
    setRefusal(null);
    void uploadModel({ hef, labels, about, use }).then((outcome) => {
      setBusy(false);
      setRefusal(outcome.refusal);
      if (outcome.message !== '') notice.show(outcome.message, null);
    });
  };
  return (
    <div className="flex flex-col gap-3 px-list py-2">
      <label className="grid gap-label">
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          Model (.hef, compiled for Hailo-8)
        </Text>
        <input type="file" accept=".hef" aria-label="Model file" className={fileInput} onChange={(event) => setHef(event.target.files?.[0] ?? null)} />
      </label>
      <label className="grid gap-label">
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          Class labels (.labels, one per line — optional). Without labels, boxes are named by class number.
        </Text>
        <input type="file" accept=".labels,.txt" aria-label="Class labels file" className={fileInput} onChange={(event) => setLabels(event.target.files?.[0] ?? null)} />
      </label>
      <OutlinedTextField label="What it's for (shown next to the model — optional)" ariaLabel="What it's for" value={about} placeholder="Aerial — vehicles at 400 ft" onValueChange={setAbout} />
      <span className="flex items-center gap-3">
        <Checkbox checked={use} label="Switch to this model after upload" onCheckedChange={setUse} />
        <Text typography="bodyMedium">Switch to this model after upload</Text>
      </span>
      <RefusalText refusal={refusal} />
      <div>
        <FilledTonalButton enabled={!busy} onClick={upload}>
          {busy ? 'Uploading…' : 'Upload model'}
        </FilledTonalButton>
      </div>
    </div>
  );
};

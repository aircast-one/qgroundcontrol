import { type ReactNode, useState } from 'react';
import { FilledTonalButton, OutlinedTextField, Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';
import { type CameraView, type ServerDest, type ShareLink, SETUP_SECTIONS, cloudflareTarget, openedSection, parseCameras } from '@/ui/DeviceCameras';
import { ActionButton, CopyRow, DeviceForm, DeviceNotice, DeviceSwitchRow, DeviceText, RefusalText, useDeviceAction } from '@/ui/DeviceKit';
import { LinkButton, LinkedNoticeView, PartDisclosure } from '@/ui/DeviceKitCamera';
import type { DeviceScreenProps } from '@/ui/DeviceScreens';
import { DeviceLevel } from '@/ui/DeviceStatus';
import { EndpointRows } from '@/ui/DeviceTelemetryScreen';
import { deviceUrl, useDeviceView } from '@/ui/DeviceView';
import { FootNote } from '@/ui/SetupRow';

type Open = (href: string) => void;

export const CameraSetupScreen = ({ href, query, onOpen, onClose }: DeviceScreenProps) => {
  const view = useDeviceView('cameras', parseCameras);
  if (view === null) return <FootNote text="Waiting for the device to answer…" />;
  const camera = view.cameras.find((it) => it.name === (query.get('camera') ?? ''));
  if (camera === undefined)
    return (
      <>
        <DeviceNotice
          notice={{
            level: DeviceLevel.Neutral,
            text: 'Pick a camera on the Cameras page to set it up.',
          }}
        />
        <div className="px-list">
          <TextButton onClick={() => onOpen('/streaming')}>Cameras →</TextButton>
        </div>
      </>
    );
  return <CameraSetup key={camera.name} camera={camera} opened={openedSection(href)} onOpen={onOpen} onClose={onClose} />;
};

const Sub = ({ text }: { text: string }) => (
  <div className="px-list pt-3 pb-1">
    <Text typography="labelLarge" color={md('onSurfaceVariant')}>
      {text}
    </Text>
  </div>
);

const Row = ({ children }: { children: ReactNode }) => <div className="flex flex-wrap items-center gap-2 px-list py-2">{children}</div>;

const CameraSetup = ({ camera, opened, onOpen, onClose }: { camera: CameraView; opened: string; onOpen: Open; onClose: () => void }) => {
  const { name, setup } = camera;
  return (
    <>
      <PartDisclosure part={setup.watch.part} id={SETUP_SECTIONS.watch} opened={opened}>
        <WatchBody camera={camera} onOpen={onOpen} />
      </PartDisclosure>
      <PartDisclosure part={setup.radio.part} id={SETUP_SECTIONS.radio} opened={opened}>
        <LinkedNoticeView notice={setup.radio.notice} onOpen={onOpen} />
        {setup.radio.form ? (
          <DeviceForm
            key={setup.radio.address}
            method="PUT"
            path={`/api/streams/${name}/wfb`}
            submit="Save"
            success={`${name} is now going over the radio.`}
            note="Takes effect immediately — no restart."
            fields={[
              {
                name: 'address',
                label: 'Address wfb_tx reads from',
                value: setup.radio.address,
                keyboard: 'uri',
                hint: 'The default is where a stock wfb-ng install reads from.',
              },
            ]}
          />
        ) : null}
        {setup.radio.stop ? (
          <Row>
            <ActionButton
              label="Stop"
              tonal={false}
              request={{ method: 'DELETE', path: `/api/streams/${name}/wfb` }}
              success={`Stopped sending ${name} over the radio.`}
              confirm={{
                title: 'Stop the radio',
                text: `Stop sending ${name} over the radio link?`,
                confirm: 'Stop',
              }}
            />
          </Row>
        ) : null}
      </PartDisclosure>
      <PartDisclosure part={setup.servers.part} id={SETUP_SECTIONS.servers} opened={opened}>
        <div className="mx-list overflow-hidden rounded-shape-md border border-outline-variant">
          {setup.servers.dests.map((dest) => (
            <PartDisclosure key={dest.kind} part={dest.part} id={`cam-dest-${dest.kind}`} opened={opened}>
              <ServerBody camera={camera} dest={dest} onOpen={onOpen} />
            </PartDisclosure>
          ))}
        </div>
      </PartDisclosure>
      <PartDisclosure part={setup.shares.part} id={SETUP_SECTIONS.shares} opened={opened}>
        {setup.shares.links.map((link) => (
          <ShareRow key={link.id} link={link} />
        ))}
        <DeviceForm
          method="POST"
          path="/api/share"
          submit="Create link"
          success="Share link created."
          note="The link opens on a page that is the picture and nothing else — no settings, no addresses."
          extra={{ camera: name }}
          fields={[
            { name: 'label', label: 'Who is it for', value: '', placeholder: 'e.g. command post' },
            {
              name: 'hours',
              label: 'Stops working after',
              value: setup.shares.hours[0]?.value ?? '24',
              options: setup.shares.hours,
            },
          ]}
        />
        <DeviceText text="A link stops working when it expires or when you turn it off. It is not a lock: anyone who can already reach this device on your network can watch the camera anyway." />
      </PartDisclosure>
      <PartDisclosure part={setup.detect.part} id={SETUP_SECTIONS.detect} opened={opened}>
        <DeviceSwitchRow
          title={`Draw detection boxes for ${name}`}
          checked={setup.detect.on}
          on={{ method: 'PUT', path: `/api/streams/${name}/detect` }}
          off={{ method: 'DELETE', path: `/api/streams/${name}/detect` }}
          success={setup.detect.on ? `Detection turned off for ${name}.` : `Detection turned on for ${name}.`}
        />
        <DeviceText text="Draw object-detection boxes over this camera. The video is untouched — boxes are sent as data and drawn by viewers, so there's no added latency. Needs the detection sidecar running on the device." />
        {setup.detect.model !== '' ? (
          <Row>
            <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
              {setup.detect.model}
            </Text>
            <TextButton onClick={() => onOpen('/settings/video')}>Change model →</TextButton>
          </Row>
        ) : null}
        {setup.detect.notice !== null ? <LinkedNoticeView notice={setup.detect.notice} onOpen={onOpen} /> : null}
      </PartDisclosure>
      <PartDisclosure part={setup.record.part} id={SETUP_SECTIONS.record} opened={opened}>
        <DeviceSwitchRow
          title={`Record ${name} to disk`}
          checked={setup.record.on}
          on={{ method: 'PUT', path: `/api/streams/${name}/record` }}
          off={{ method: 'DELETE', path: `/api/streams/${name}/record` }}
          success={setup.record.on ? `Recording turned off for ${name} — download it under Settings → Recordings.` : `Recording turned on for ${name}.`}
        />
        <DeviceText text="Recorded in ten-minute segments to this device's storage, and playable as soon as a segment closes. The oldest go when the size limit is reached, so recording can never fill the card and take the device down with it." />
        {setup.record.full ? (
          <DeviceNotice
            notice={{
              level: DeviceLevel.Warn,
              text: 'Recordings have reached the size limit, so the oldest are being deleted to make room for new ones.',
            }}
          />
        ) : null}
        {setup.record.usage !== '' ? (
          <Row>
            <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
              {setup.record.usage}
            </Text>
            <TextButton onClick={() => onOpen('/settings/recordings')}>Open recordings →</TextButton>
          </Row>
        ) : null}
      </PartDisclosure>
      {setup.gimbal !== null ? (
        <PartDisclosure part={setup.gimbal.part} id={SETUP_SECTIONS.gimbal} opened={opened}>
          {setup.gimbal.owner ? (
            <>
              <DeviceText text="Tracking from QGroundControl steers the gimbal from this camera's picture. Only one camera can be on the gimbal — naming another one moves tracking there." />
              <Row>
                <ActionButton label="Not this camera" tonal={false} request={{ method: 'DELETE', path: `/api/streams/${name}/gimbal/owner` }} success={`Tracking turned off for ${name}.`} />
              </Row>
            </>
          ) : (
            <>
              <DeviceText text="This device has a gimbal, but it is not this camera's, so QGroundControl cannot track from this picture. Say it is the gimbal camera to track from it." />
              <Row>
                <ActionButton label="This is the gimbal camera" request={{ method: 'PUT', path: `/api/streams/${name}/gimbal/owner` }} success={`${name} is now the gimbal camera.`} />
              </Row>
            </>
          )}
        </PartDisclosure>
      ) : null}
      <div className="flex justify-end px-list py-4">
        <ActionButton
          label="Remove this camera"
          tonal={false}
          request={{ method: 'DELETE', path: `/api/streams/${name}` }}
          success={`Camera ${name} removed.`}
          confirm={{
            title: 'Remove camera',
            text: `Remove camera ${name}? Its stream URLs stop working, and once no other camera uses its port the port returns to the rescue network.`,
            confirm: 'Remove',
          }}
          onDone={onClose}
        />
      </div>
    </>
  );
};

const WatchBody = ({ camera, onOpen }: { camera: CameraView; onOpen: Open }) => {
  const { name } = camera;
  const watch = camera.setup.watch;
  return (
    <>
      <Sub text="From anywhere" />
      {watch.cloud ? (
        <>
          <EndpointRows endpoints={watch.cloudLinks} />
          <DeviceText text={watch.cloudCopy} />
          {watch.signIn ? (
            <LinkedNoticeView
              notice={{
                level: DeviceLevel.Warn,
                text: 'Add cloud credentials in Engineering → Publishing sign-in to finish authorizing the stream.',
                link: { label: 'Engineering', href: '/settings/engineering' },
              }}
              onOpen={onOpen}
            />
          ) : null}
          <Row>
            <ActionButton
              label="Disconnect"
              tonal={false}
              request={{ method: 'DELETE', path: `/api/stream/sfu/${name}/cloud` }}
              success={`${name} disconnected from the cloud.`}
              confirm={{
                title: 'Disconnect from the cloud',
                text: `Disconnect ${name} from the Aircast cloud? Its cloud watch page stops working. Share links keep working on this network.`,
                confirm: 'Disconnect',
              }}
            />
          </Row>
        </>
      ) : watch.canConnect ? (
        <>
          <DeviceText text="Optional: a watch page on the Aircast cloud, reachable from outside this network." />
          <Row>
            <ActionButton label="Connect to cloud" request={{ method: 'POST', path: `/api/stream/sfu/${name}/cloud` }} success={`Connecting ${name} to the Aircast cloud…`} />
          </Row>
        </>
      ) : (
        <LinkedNoticeView
          notice={{
            level: DeviceLevel.Neutral,
            text: 'Needs a cloud account on this device — Device → Cloud account.',
            link: { label: 'Device', href: '/settings/device' },
          }}
          onOpen={onOpen}
        />
      )}
      <Sub text="On this network" />
      <EndpointRows endpoints={watch.links} />
      <Row>
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          Didn't play, or the ground station is another machine?
        </Text>
        <TextButton onClick={() => onOpen('/connect')}>Ground station links →</TextButton>
      </Row>
    </>
  );
};

const STOP_LABELS: Readonly<Record<string, string>> = {
  whip: 'the SFU',
  livekit: 'LiveKit',
  cloudflare: 'Cloudflare',
};

const ServerBody = ({ camera, dest, onOpen }: { camera: CameraView; dest: ServerDest; onOpen: Open }) => {
  const { name } = camera;
  const path = `/api/stream/sfu/${name}/${dest.kind}`;
  const label = STOP_LABELS[dest.kind] ?? dest.kind;
  return (
    <>
      {dest.kind === 'whip' ? (
        <>
          <DeviceForm
            key={dest.url}
            method="PUT"
            path={path}
            submit="Save"
            success={`Publishing ${name} to the SFU.`}
            fields={[
              {
                name: 'url',
                label: 'WHIP URL',
                value: dest.url,
                placeholder: 'https://sfu.example.com/whip/device',
                keyboard: 'uri',
              },
            ]}
          />
          <Row>
            <Text typography="bodySmall" color={md('onSurfaceVariant')}>
              Signs in with the device's sign-in method.
            </Text>
            <TextButton onClick={() => onOpen('/settings/engineering')}>Publishing sign-in →</TextButton>
          </Row>
        </>
      ) : null}
      {dest.kind === 'livekit' ? (
        <DeviceForm
          key={`${dest.url}|${dest.key}|${dest.secret}`}
          method="PUT"
          path={path}
          submit="Save"
          success={`Publishing ${name} to LiveKit.`}
          note="One key pair is shared by every camera published to LiveKit — changing it here changes it everywhere."
          fields={[
            {
              name: 'url',
              label: 'Server URL',
              value: dest.url,
              placeholder: 'wss://livekit.example.com',
              keyboard: 'uri',
              hint: "The room is this camera's name.",
            },
            { name: 'livekitKey', label: 'API key', value: dest.key, placeholder: 'APIxxxxxxxx' },
            {
              name: 'livekitSecret',
              label: 'API secret',
              value: dest.secret,
              placeholder: 'LiveKit API secret',
              keyboard: 'password',
            },
          ]}
        />
      ) : null}
      {dest.kind === 'cloudflare' ? <CloudflareForm key={`${dest.app}|${dest.track}|${dest.secret}`} name={name} dest={dest} /> : null}
      {dest.on ? (
        <Row>
          <ActionButton
            label="Stop publishing"
            tonal={false}
            request={{ method: 'DELETE', path }}
            success={`${name} stopped publishing to ${label}.`}
            confirm={{
              title: 'Stop publishing',
              text: `Stop publishing ${name} to ${label}?`,
              confirm: 'Stop',
            }}
          />
        </Row>
      ) : null}
      {dest.kind === 'cloudflare' && dest.on ? (
        <>
          {dest.watch !== '' ? (
            <Row>
              <LinkButton href={deviceUrl(dest.watch)} filled>
                ▶ Watch via Cloudflare
              </LinkButton>
            </Row>
          ) : null}
          <DeviceText
            text={`This player pulls the stream back from Cloudflare — proof of what your viewers get. There's no public watch URL: a viewer app pulls track ${dest.track} from app ${dest.app} through Cloudflare's Sessions API, authorized with the app secret.`}
          />
        </>
      ) : null}
    </>
  );
};

const CloudflareForm = ({ name, dest }: { name: string; dest: ServerDest }) => {
  const action = useDeviceAction();
  const [app, setApp] = useState(dest.app);
  const [secret, setSecret] = useState(dest.secret);
  const [track, setTrack] = useState(dest.track);
  const urlRefused = action.refusal?.field === 'url';
  return (
    <div className="flex flex-col gap-3 px-list py-2">
      <div className="flex flex-wrap items-center gap-2">
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          In the Cloudflare dashboard, open Realtime → SFU and create an app — it gives you the App ID and App secret.
        </Text>
        <LinkButton href="https://dash.cloudflare.com/">Cloudflare dashboard</LinkButton>
      </div>
      <OutlinedTextField
        label="App ID"
        value={app}
        placeholder="Realtime → SFU → your app"
        autoCorrect={false}
        isError={urlRefused}
        supportingText={urlRefused ? action.refusal?.message : undefined}
        onValueChange={setApp}
      />
      <OutlinedTextField
        label="App secret"
        value={secret}
        placeholder="Realtime app secret"
        keyboardType="password"
        isError={action.refusal?.field === 'secret'}
        supportingText={action.refusal?.field === 'secret' ? action.refusal.message : undefined}
        onValueChange={setSecret}
      />
      <OutlinedTextField
        label="Track name"
        value={track}
        autoCorrect={false}
        supportingText="Not from Cloudflare — a name you pick. Viewers subscribe to this stream by this name. The camera name works fine."
        onValueChange={setTrack}
      />
      <RefusalText refusal={action.refusal !== null && (action.refusal.field === 'url' || action.refusal.field === 'secret') ? null : action.refusal} field={action.refusal?.field ?? ''} />
      <div>
        <FilledTonalButton
          enabled={!action.busy}
          onClick={() =>
            void action.run(
              {
                method: 'PUT',
                path: `/api/stream/sfu/${name}/cloudflare`,
                body: { url: cloudflareTarget(app, track), secret },
              },
              `Publishing ${name} to Cloudflare.`,
            )
          }
        >
          Save
        </FilledTonalButton>
      </div>
    </div>
  );
};

const ShareRow = ({ link }: { link: ShareLink }) =>
  link.dead === '' ? (
    <>
      <CopyRow label={link.label} text={link.url} hint={link.expires} qr />
      <Row>
        <ActionButton
          label="Turn off"
          tonal={false}
          request={{ method: 'DELETE', path: `/api/share/${link.id}` }}
          success="Link turned off."
          confirm={{
            title: 'Turn off this link',
            text: 'Turn off this link? Whoever has it stops being able to open the picture.',
            confirm: 'Turn off',
          }}
        />
      </Row>
    </>
  ) : (
    <div className="flex flex-col px-list py-2 opacity-70">
      <Text typography="bodyLarge">{link.label}</Text>
      <Text typography="bodySmall" color={md('onSurfaceVariant')}>
        {link.dead}
      </Text>
    </div>
  );

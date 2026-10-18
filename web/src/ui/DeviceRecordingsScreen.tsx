import { useState } from 'react';
import { IconButton, Text } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { ActionButton, ConfirmDialog, DeviceNotice, DeviceSection, DeviceSelect, DeviceText, RefusalText, useDeviceAction } from '@/ui/DeviceKit';
import { LinkButton, SettingSwitchRow, StatusLine } from '@/ui/DeviceKitSettings';
import { type RecordedCamera, type RecordingSegment, type RecordingsLimit, lowersLimit, parseRecordings, recordPath, recordingsPath, segmentDetail, segmentPath } from '@/ui/DeviceRecordings';
import type { DeviceScreenProps } from '@/ui/DeviceScreens';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { deviceUrl, useDeviceView } from '@/ui/DeviceView';
import { FootNote } from '@/ui/SetupRow';

export const RecordingsScreen = (_: DeviceScreenProps) => {
  const view = useDeviceView('recordings', parseRecordings);
  if (view === null) return <FootNote text="Waiting for the device to answer…" />;
  return (
    <>
      <DeviceSection title="Storage">
        <LimitPicker key={view.limit.value} limit={view.limit} />
        <StatusLine text="Recordings share the card this device boots from. Past the limit the oldest go first, so recording can never fill the card." />
      </DeviceSection>
      {view.cameras.length > 0 ? (
        <DeviceSection title="Cameras">
          {view.cameras.map((camera) => (
            <SettingSwitchRow
              key={camera.name}
              title={camera.name}
              detail={camera.note}
              checked={camera.recording}
              on={{ method: 'PUT', path: recordPath(camera.name) }}
              off={{ method: 'DELETE', path: recordPath(camera.name) }}
              onSuccess={`Recording turned on for ${camera.name}.`}
              offSuccess={`Recording turned off for ${camera.name} — its recordings stay below.`}
            />
          ))}
        </DeviceSection>
      ) : null}
      {view.recorded.length > 0 ? (
        view.recorded.map((camera) => <RecordedSection key={camera.name} camera={camera} />)
      ) : (
        <DeviceSection title="On this device">
          <DeviceText text="Nothing recorded yet. Turn recording on for a camera above." />
        </DeviceSection>
      )}
    </>
  );
};

const LimitPicker = ({ limit }: { limit: RecordingsLimit }) => {
  const action = useDeviceAction();
  const [picked, setPicked] = useState(limit.value);
  const [lowering, setLowering] = useState<string | null>(null);
  const save = (value: string) => {
    setPicked(value);
    void action.run({ method: 'PUT', path: '/api/recordings/limits', body: { maxBytes: value } }, 'Recording limit saved.').then((done) => (done ? undefined : setPicked(limit.value)));
  };
  return (
    <div className="flex flex-col gap-1 px-list py-2">
      <div className="max-w-xs">
        <DeviceSelect label="Recordings may use" value={picked} options={limit.options} enabled={!action.busy} onPick={(value) => (lowersLimit(limit.value, value) ? setLowering(value) : save(value))} />
      </div>
      {limit.used !== '' ? (
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          {limit.used}
        </Text>
      ) : null}
      <RefusalText refusal={action.refusal} field={action.refusal?.field ?? ''} />
      {lowering !== null ? (
        <ConfirmDialog
          title="Lower the limit?"
          text="Lowering the limit deletes the oldest recordings straight away."
          confirm="Lower it"
          onConfirm={() => save(lowering)}
          onDismiss={() => setLowering(null)}
        />
      ) : null}
    </div>
  );
};

const RecordedSection = ({ camera }: { camera: RecordedCamera }) => (
  <DeviceSection title={camera.name} badge={camera.status}>
    {camera.full ? <DeviceNotice notice={{ level: DeviceLevel.Warn, text: 'Recordings have reached the size limit, so the oldest are being deleted to make room for new ones.' }} /> : null}
    <div className="flex flex-wrap items-center justify-between gap-2 px-list py-1">
      <Text typography="bodySmall" color={md('onSurfaceVariant')}>
        {`${camera.summary} · ${camera.usage}`}
      </Text>
      <ActionButton
        label="Delete all"
        tonal={false}
        request={{ method: 'DELETE', path: recordingsPath(camera.name) }}
        success={`Deleted every recording of ${camera.name}.`}
        confirm={{ title: 'Delete every recording?', text: `Delete every recording of ${camera.name}? This cannot be undone.`, confirm: 'Delete all' }}
      />
    </div>
    <ul aria-label={`Recordings of ${camera.name}`} className="flex flex-col">
      {camera.segments.map((segment) => (
        <SegmentRow key={segment.at} camera={camera.name} segment={segment} />
      ))}
    </ul>
    <StatusLine text="Each download carries the aircraft's height, speed, battery and position as a subtitle track. Players keep subtitles off until asked — in QuickTime choose View → Subtitles, in VLC Subtitle → Sub Track." />
  </DeviceSection>
);

const SegmentRow = ({ camera, segment }: { camera: string; segment: RecordingSegment }) => {
  const action = useDeviceAction();
  const [asking, setAsking] = useState(false);
  return (
    <li className="flex flex-col" aria-label={`Recording from ${segment.started}`}>
      <div className="flex items-center gap-2 px-list py-1.5">
        <div className="flex min-w-0 flex-1 flex-col">
          <Text typography="bodyLarge">{segment.started}</Text>
          <Text typography="bodySmall" color={md('onSurfaceVariant')}>
            {segmentDetail(segment).join(' · ')}
          </Text>
        </div>
        {segment.open ? null : (
          <LinkButton href={deviceUrl(segment.url)} download tonal={false}>
            Download
          </LinkButton>
        )}
        <IconButton label={`Delete the recording from ${segment.started}`} enabled={!action.busy} onClick={() => setAsking(true)}>
          <MaterialIcon name="delete" />
        </IconButton>
      </div>
      <RefusalText refusal={action.refusal} field={action.refusal?.field ?? ''} />
      {asking ? (
        <ConfirmDialog
          title="Delete this recording?"
          text={`Delete the recording from ${segment.started}?`}
          confirm="Delete"
          onConfirm={() => void action.run({ method: 'DELETE', path: segmentPath(camera, segment) }, 'Recording deleted.')}
          onDismiss={() => setAsking(false)}
        />
      ) : null}
    </li>
  );
};

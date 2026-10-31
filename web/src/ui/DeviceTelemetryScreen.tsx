import { useProduceState } from '@/kotlin/Compose';
import { FilledTonalButton, IconButton, OutlinedTextField, Text, TextButton } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { useDeviceDraft } from '@/ui/DeviceDraft';
import { CopyRow, DeviceBadgeText, DeviceColumns, DeviceForm, DeviceNotice, DeviceSection, DeviceSelect, DeviceText, RefusalText, useDeviceAction } from '@/ui/DeviceKit';
import type { DeviceScreenProps } from '@/ui/DeviceScreens';
import { DeviceLevel } from '@/ui/DeviceLevel';
import {
  type DeviceEndpoint,
  type EndpointEditor,
  type EndpointRow,
  type EndpointRowView,
  type GimbalSettings,
  type TelemetryView,
  endpointEditor,
  parseTelemetry,
} from '@/ui/DeviceTelemetry';
import { useDeviceView } from '@/ui/DeviceView';
import { FootNote } from '@/ui/SetupRow';

export const endpointLabel = (endpoint: DeviceEndpoint): string => (endpoint.proto !== '' && !endpoint.label.startsWith(endpoint.proto) ? `${endpoint.proto} · ${endpoint.label}` : endpoint.label);

export const EndpointRows = ({ endpoints }: { endpoints: readonly DeviceEndpoint[] }) => (
  <>
    {endpoints.map((endpoint) => (
      <CopyRow
        key={endpoint.url}
        label={endpointLabel(endpoint)}
        text={endpoint.url}
        hint={[endpoint.hint, endpoint.note].filter((it) => it !== '').join(' ')}
        qr
        level={endpoint.down ? DeviceLevel.Warn : DeviceLevel.Neutral}
      />
    ))}
  </>
);

export const TelemetryScreen = (_: DeviceScreenProps) => {
  const view = useDeviceView('telemetry', parseTelemetry);
  if (view === null) return <FootNote text="Waiting for the device to answer…" />;
  return (
    <DeviceColumns
      left={
        <>
          <DeviceSection title="Status" badge={view.status}>
            {view.notices.map((notice) => (
              <DeviceNotice key={notice.text} notice={notice} />
            ))}
            {view.cloud !== null ? (
              <div className="px-list py-2" role="group" aria-label="Cloud path">
                <div className="flex items-center justify-between gap-3">
                  <Text typography="bodyLarge">Cloud path</Text>
                  <DeviceBadgeText badge={view.cloud.status} />
                </div>
                <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
                  {view.cloud.text}
                </Text>
              </div>
            ) : null}
          </DeviceSection>
          <DeviceSection title="Connect another ground station">
            <EndpointRows endpoints={view.endpoints} />
            {view.endpointNotice !== null ? <DeviceNotice notice={view.endpointNotice} /> : null}
          </DeviceSection>
        </>
      }
      right={
        <>
          <DeviceSection title="MAVLink settings">
            <MavlinkEditor view={view} />
          </DeviceSection>
          {view.gimbal !== null ? <GimbalSection gimbal={view.gimbal} /> : null}
        </>
      }
    />
  );
};

const MavlinkEditor = ({ view }: { view: TelemetryView }) => {
  const stored = useProduceState<EndpointEditor | null>(null, [view.settings.endpoints], async (set) => set(await endpointEditor(view.schemes, null, view.settings.endpoints)));
  const typed = useDeviceDraft<readonly EndpointRow[] | null>({ rows: stored?.rows.map((it) => it.row) ?? null });
  const typedBaud = useDeviceDraft({ baud: view.settings.baud });
  const action = useDeviceAction();
  const rows = typed.values.rows ?? null;
  const baud = typedBaud.values.baud ?? view.settings.baud;
  const editor = useProduceState<EndpointEditor | null>(null, [JSON.stringify(rows)], async (set) => set(rows === null ? null : await endpointEditor(view.schemes, rows, view.settings.endpoints)));
  if (rows === null || editor === null) return null;
  const setRows = (next: readonly EndpointRow[]) => typed.edit('rows', next);
  const change = (index: number, next: EndpointRow) => setRows(rows.map((row, at) => (at === index ? next : row)));
  const save = async () => {
    const saved = await endpointEditor(view.schemes, rows, view.settings.endpoints);
    if (saved === null || !(await action.run({ method: 'PUT', path: '/api/telemetry/config', body: { endpoints: saved.endpoints, baud } }, 'MAVLink settings applied.'))) return;
    typed.accept({ rows });
    typedBaud.accept({ baud });
  };
  return (
    <div className="flex flex-col gap-3 px-list py-2">
      <Text typography="bodyLarge">Ground-station endpoints</Text>
      {rows.map((row, index) => (
        <EndpointEditorRow
          key={index}
          row={row}
          shown={editor.rows[index] ?? editor.blank}
          onChange={(next) => change(index, next)}
          onRemove={() => setRows(rows.filter((_, at) => at !== index))}
        />
      ))}
      <div>
        <TextButton onClick={() => setRows([...rows, editor.blank.row])}>+ Add endpoint</TextButton>
      </div>
      <OutlinedTextField
        label="Serial baud"
        value={baud}
        placeholder="115200"
        keyboardType="number"
        className="max-w-md"
        isError={action.refusal?.field === 'baud'}
        supportingText={action.refusal?.field === 'baud' ? action.refusal.message : 'The baud only matters for a serial flight controller — USB autopilots ignore it, Betaflight ships at 115200.'}
        onValueChange={(value) => typedBaud.edit('baud', value)}
      />
      <Text typography="bodySmall" color={md('onSurfaceVariant')}>
        Takes effect on save — no reboot.
      </Text>
      <RefusalText refusal={action.refusal?.field === 'baud' ? null : action.refusal} field={action.refusal?.field ?? ''} />
      <div>
        <FilledTonalButton enabled={!action.busy} onClick={() => void save()}>
          Save
        </FilledTonalButton>
      </div>
    </div>
  );
};

const EndpointEditorRow = ({ row, shown, onChange, onRemove }: { row: EndpointRow; shown: EndpointRowView; onChange: (row: EndpointRow) => void; onRemove: () => void }) => (
  <div className="flex flex-col gap-2 rounded-shape-md bg-surface-container-high p-3" role="group" aria-label="Endpoint">
    <div className="flex flex-wrap items-end gap-2">
      <div className="w-48 max-sm:w-full">
        <DeviceSelect label="Connection type" value={row.scheme} options={shown.options} onPick={(scheme) => onChange(shown.options.find((it) => it.value === scheme)?.row ?? row)} />
      </div>
      <OutlinedTextField
        label={shown.hostLabel}
        ariaLabel={shown.hostLabel}
        value={row.host}
        placeholder={shown.placeholder}
        keyboardType="uri"
        autoCorrect={false}
        className="min-w-40 flex-1"
        onValueChange={(host) => onChange({ ...row, host })}
      />
      <OutlinedTextField label="Port" ariaLabel="Port" value={row.port} placeholder="14550" keyboardType="number" className="w-28" onValueChange={(port) => onChange({ ...row, port })} />
      <IconButton label="Remove endpoint" onClick={onRemove}>
        <MaterialIcon name="close" />
      </IconButton>
    </div>
    {shown.problem !== '' ? (
      <Text typography="bodySmall" color={md('error')}>
        {shown.problem}
      </Text>
    ) : (
      <Text typography="bodySmall" color={md('onSurfaceVariant')}>
        {shown.note}
      </Text>
    )}
  </div>
);

const GimbalSection = ({ gimbal }: { gimbal: GimbalSettings }) => (
  <DeviceSection title={gimbal.title} badge={gimbal.status}>
    <DeviceText text={gimbal.about} />
    <DeviceForm
      method="PUT"
      path="/api/gimbal/config"
      submit="Save"
      success="Gimbal settings applied."
      note="Takes effect on save — no reboot. Changing the make resets the model and address to that make's defaults if the old ones do not fit."
      fields={[
        { name: 'vendor', label: 'Make', value: gimbal.vendor, options: gimbal.vendors },
        { name: 'model', label: 'Model', value: gimbal.model, options: gimbal.models },
        { name: 'cameraAddr', label: 'Camera address', value: gimbal.cameraAddr, placeholder: '192.168.144.25:37260', keyboard: 'uri' },
        { name: 'mavlinkAddr', label: 'MAVLink endpoint', value: gimbal.mavlinkAddr, placeholder: '127.0.0.1:14550', keyboard: 'uri' },
      ]}
    />
  </DeviceSection>
);

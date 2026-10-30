import { type ReactNode, useContext, useEffect, useMemo, useState } from 'react';
import { AccountCommands, LinkCommands } from '@/bridge/BridgeCommands';
import { type Json, jsonObjects, jsonStrings, optBoolean, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath, useQgcValue } from '@/bridge/QgcState';
import { stageFile, unstage, unstageAll } from '@/core/files';
import { useBackHandler, useLaunchedEffect, useTicker } from '@/kotlin/Compose';
import { openDocument } from '@/kotlin/Platform';
import { useValue } from '@/kotlin/State';
import { delay, same } from '@/kotlin/Std';
import { AlertDialog, Button, Checkbox, DropdownMenu, DropdownMenuItem, FilterChip, IconButton, OutlinedButton, OutlinedTextField, Switch, Text, TextButton } from '@/m3';
import { type DrawableName } from '@/res/drawables';
import { Drawable, MaterialIcon } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';
import { AIRCAST_CLOUD_NAME, AircastCloudFields, DEFAULT_ACCOUNT_SERVER, accountState, cloudApiBaseValid, cloudDeviceValid } from '@/ui/AircastCloud';
import { LocalAppNavigation } from '@/ui/AppNavigation';
import { useHasVehicle } from '@/ui/FlyState';
import { MockLinkChoices, MockLinkFields, mockLinkArguments, preferredMockChoices } from '@/ui/MockLinkFields';
import { ChoiceField } from '@/ui/SettingsScreen';
import { AdvancedToggle, FootNote, LocalPageHeading, OverridePageHeading, PageTopBar, SectionHeader, SetupRow } from '@/ui/SetupRow';
import { USB_RADIO_PORT, UsbAccess } from '@/ui/UsbAccess';
import { serialAvailable } from '@/core/client';

export const LINKS_VIEW = 'view.links';
const LINKS_PATH = 'links.linkConfigurations';
const DEFAULT_PORT = '14550';
const DEFAULT_TCP_PORT = '5760';
const UDP_LISTEN_PORT = 'settings.autoConnectSettings.udpListenPort';

export type LinkType = {
  readonly name: 'Udp' | 'Tcp' | 'Serial' | 'Bluetooth' | 'LogReplay' | 'Mock' | 'AircastCloud' | 'WebSocket' | 'Other';
  readonly ordinal: number;
  readonly id: string;
};

export const LinkType = {
  Udp: { name: 'Udp', ordinal: 0, id: 'udp' },
  Tcp: { name: 'Tcp', ordinal: 1, id: 'tcp' },
  Serial: { name: 'Serial', ordinal: 2, id: 'serial' },
  Bluetooth: { name: 'Bluetooth', ordinal: 3, id: 'bluetooth' },
  LogReplay: { name: 'LogReplay', ordinal: 4, id: 'logReplay' },
  Mock: { name: 'Mock', ordinal: 5, id: 'mock' },
  AircastCloud: { name: 'AircastCloud', ordinal: 6, id: 'aircastCloud' },
  WebSocket: { name: 'WebSocket', ordinal: 7, id: 'websocket' },
  Other: { name: 'Other', ordinal: 8, id: '' },
} as const satisfies Record<string, LinkType>;

export const LinkTypeEntries: readonly LinkType[] = [LinkType.Udp, LinkType.Tcp, LinkType.Serial, LinkType.Bluetooth, LinkType.LogReplay, LinkType.Mock, LinkType.AircastCloud, LinkType.WebSocket, LinkType.Other];

export const linkTypeFrom = (id: string): LinkType => LinkTypeEntries.find((it) => it.id === id && it !== LinkType.Other) ?? LinkType.Other;

export type LinkEditing = {
  readonly name: 'HostAndPort' | 'PortOnly' | 'Serial' | 'LogFile' | 'Device' | 'None';
  readonly ordinal: number;
  readonly id: string;
};

export const LinkEditing = {
  HostAndPort: { name: 'HostAndPort', ordinal: 0, id: 'hostAndPort' },
  PortOnly: { name: 'PortOnly', ordinal: 1, id: 'portOnly' },
  Serial: { name: 'Serial', ordinal: 2, id: 'serial' },
  LogFile: { name: 'LogFile', ordinal: 3, id: 'logFile' },
  Device: { name: 'Device', ordinal: 4, id: 'device' },
  None: { name: 'None', ordinal: 5, id: '' },
} as const satisfies Record<string, LinkEditing>;

export const LinkEditingEntries: readonly LinkEditing[] = [LinkEditing.HostAndPort, LinkEditing.PortOnly, LinkEditing.Serial, LinkEditing.LogFile, LinkEditing.Device, LinkEditing.None];

export const linkEditingFrom = (id: string): LinkEditing => LinkEditingEntries.find((it) => it.id === id && it !== LinkEditing.None) ?? LinkEditing.None;

export type SerialFraming = { dataBits: number; stopBits: number; parity: number; flowControl: number };

export const SerialFraming = (fields: Partial<SerialFraming> = {}): SerialFraming => ({ dataBits: 8, stopBits: 1, parity: 0, flowControl: 0, ...fields });

export type LinkRow = {
  index: number;
  name: string;
  statusLine: string;
  goneQuiet: boolean;
  connected: boolean;
  heard: boolean;
  lastError: string;
  errorRemedy: string;
  editing: LinkEditing;
  host: string;
  port: number;
  portName: string;
  baud: number;
  framing: SerialFraming;
  servers: readonly string[];
  autoConnect: boolean;
  highLatency: boolean;
  type: LinkType;
  filename: string;
};

export const LinkRow = (fields: Pick<LinkRow, 'index' | 'name' | 'statusLine' | 'connected' | 'heard' | 'lastError'> & Partial<LinkRow>): LinkRow => ({
  goneQuiet: false,
  errorRemedy: '',
  editing: LinkEditing.None,
  host: '',
  port: 0,
  portName: '',
  baud: 0,
  framing: SerialFraming(),
  servers: [],
  autoConnect: false,
  highLatency: false,
  type: LinkType.Other,
  filename: '',
  ...fields,
});

export type AutoLink = { name: string; summary: string; heard: boolean; type: LinkType; index: number };

export const autoLinks = (view: Json | null): readonly AutoLink[] =>
  jsonObjects(optJSONArray(view, 'links'))
    .filter((it) => optBoolean(it, 'dynamic') && optBoolean(it, 'connected'))
    .map((it) => ({ name: optText(it, 'name'), summary: optText(it, 'displaySummary'), heard: optBoolean(it, 'heardVehicle'), type: linkTypeFrom(optText(it, 'type')), index: optInt(it, 'index') }));

export const autoLinkStatus = (link: AutoLink): string => (link.heard ? 'Vehicle' : 'Listening');

export const autoLinkSubtitle = (link: AutoLink): string =>
  link.type === LinkType.Mock ? 'Simulated' : [link.summary, 'automatic'].filter((it) => it.trim() !== '').join(' · ');

export const linkRows = (view: Json | null): readonly LinkRow[] =>
  jsonObjects(optJSONArray(view, 'configured')).map((link) =>
    LinkRow({
      index: optInt(link, 'index'),
      name: optText(link, 'name'),
      statusLine: optText(link, 'statusLine'),
      goneQuiet: optBoolean(link, 'goneQuiet'),
      connected: optBoolean(link, 'connected'),
      heard: optBoolean(link, 'heardVehicle'),
      lastError: optText(link, 'lastError'),
      errorRemedy: optText(link, 'errorRemedy'),
      editing: linkEditingFrom(optText(link, 'editing')),
      host: optText(link, 'host'),
      port: optInt(link, 'port'),
      portName: optText(link, 'portName'),
      baud: optInt(link, 'baud'),
      framing: SerialFraming({ dataBits: optInt(link, 'dataBits', 8), stopBits: optInt(link, 'stopBits', 1), parity: optInt(link, 'parity', 0), flowControl: optInt(link, 'flowControl', 0) }),
      servers: jsonStrings(optJSONArray(link, 'hostList')),
      autoConnect: optBoolean(link, 'autoConnect'),
      highLatency: optBoolean(link, 'highLatency'),
      type: linkTypeFrom(optText(link, 'type')),
      filename: optText(link, 'filename'),
    }),
  );

const LINK_ICONS: Record<LinkType['name'], DrawableName> = {
  Serial: 'ic_usb',
  Udp: 'ic_wifi',
  Bluetooth: 'ic_bluetooth',
  Tcp: 'ic_lan',
  AircastCloud: 'ic_cloud',
  LogReplay: 'ic_history',
  Mock: 'ic_science',
  WebSocket: 'ic_lan',
  Other: 'ic_link',
};

export const linkIcon = (type: LinkType): DrawableName => LINK_ICONS[type.name];

const REPLAY_LINK_FOLDER = 'replay-links';

const fileSafe = (text: string): string => text.replace(/[^A-Za-z0-9._-]/g, '_');

const leafName = (path: string): string => path.slice(path.lastIndexOf('/') + 1);

const replayFolder = (link: string): string => `/${REPLAY_LINK_FOLDER}/${fileSafe(link)}/`;

export const stagedReplayLog = async (link: string, file: File): Promise<string | null> => {
  const cleaned = file.name.replace(/[/\\]/g, '_');
  const path = `${replayFolder(link)}${cleaned.trim() === '' ? 'replay.tlog' : cleaned}`;
  return (await stageFile(path, file).catch(() => false)) ? path : null;
};

export const pruneReplayFolder = (link: string, keep: string): Promise<void> => unstageAll(replayFolder(link), keep);

const discardStagedLog = async (staged: string): Promise<void> => void (await unstage(staged));

export const linkIsEditable = (row: LinkRow): boolean => !row.connected && row.editing !== LinkEditing.None;

export const CREATABLE_LINK_TYPES: readonly LinkType[] = [LinkType.Udp, LinkType.Tcp, LinkType.Serial];
export const REPLAY_LINK_NAME = 'Log Replay';

export type BluetoothDeviceChoice = { name: string; address: string };

export type BluetoothState = { available: boolean; scanning: boolean; devices: readonly BluetoothDeviceChoice[] };

export const bluetoothState = (view: Json | null): BluetoothState => {
  const bluetooth = optJSONObject(view, 'bluetooth');
  return {
    available: optBoolean(bluetooth, 'available'),
    scanning: optBoolean(bluetooth, 'scanning'),
    devices: jsonObjects(optJSONArray(bluetooth, 'devices')).map((it) => ({ name: optText(it, 'name'), address: optText(it, 'address') })),
  };
};

export const linkTypeIds = (view: Json | null): ReadonlySet<LinkType> => new Set(jsonStrings(optJSONArray(view, 'linkTypeIds')).map(linkTypeFrom));

const SOCKET_LINK_TYPES: ReadonlySet<LinkType> = new Set([LinkType.Udp, LinkType.Tcp]);

const servedLinkTypes = (view: Json | null): readonly LinkType[] => {
  if (optJSONArray(view, 'linkTypeIds') === null) return CREATABLE_LINK_TYPES;
  const served = linkTypeIds(view);
  const creatable = CREATABLE_LINK_TYPES.filter((it) => served.has(it));
  const bluetooth = [LinkType.Bluetooth].filter((it) => served.has(it) && bluetoothState(view).available);
  return [...(creatable.length === 0 ? CREATABLE_LINK_TYPES : creatable), ...bluetooth, ...[LinkType.LogReplay, LinkType.Mock].filter((it) => served.has(it))];
};

export const addableLinkTypes = (view: Json | null): readonly LinkType[] => [LinkType.WebSocket, ...servedLinkTypes(view).filter((it) => optBoolean(view, 'throughDevice') || !SOCKET_LINK_TYPES.has(it))];

export const PARITY_CHOICES: readonly (readonly [string, number])[] = [
  ['None', 0],
  ['Even', 2],
  ['Odd', 3],
];
export const DATA_BITS_CHOICES: readonly number[] = [5, 6, 7, 8];
export const STOP_BITS_CHOICES: readonly number[] = [1, 2];

export type LinkWrite = readonly [string, string | number | boolean];

export const editWrites = (
  editing: LinkEditing,
  name: string,
  host: string,
  port: number,
  portName: string,
  baud: number,
  autoConnect: boolean,
  highLatency: boolean,
  framing: SerialFraming = SerialFraming(),
  logFile = '',
): readonly LinkWrite[] => {
  const common: readonly LinkWrite[] = [
    ['name', name],
    ['autoConnect', autoConnect],
    ['highLatency', highLatency],
  ];
  const specific: readonly LinkWrite[] =
    editing === LinkEditing.HostAndPort
      ? [
          ['host', host],
          ['port', port],
        ]
      : editing === LinkEditing.PortOnly
        ? [['localPort', port]]
        : editing === LinkEditing.LogFile
          ? [['filename', logFile]]
          : editing === LinkEditing.Serial
            ? [
                ['portName', portName],
                ['baud', baud],
                ['dataBits', framing.dataBits],
                ['stopBits', framing.stopBits],
                ['parity', framing.parity],
                ['flowControl', framing.flowControl],
              ]
            : [];
  return [...common, ...specific];
};

export const linkTypeLabel = (type: LinkType): string => {
  if (type === LinkType.Serial) return 'Serial';
  if (type === LinkType.Bluetooth) return 'Bluetooth';
  if (type === LinkType.AircastCloud) return 'Aircast Cloud';
  if (type === LinkType.LogReplay) return 'Log replay';
  if (type === LinkType.Mock) return 'Simulated';
  return type.id.toUpperCase();
};

export const autoLinkName = (type: LinkType, host: string, port: string): string => {
  if (type === LinkType.Udp) return `UDP ${port}`;
  if (host.trim() === '') return type.id.toUpperCase();
  return `${type.id.toUpperCase()} ${host}:${port}`;
};

export const uniqueLinkName = (base: string, taken: readonly string[]): string =>
  [base, ...Array.from({ length: taken.length + 1 }, (_, at) => `${base} (${at + 2})`)].find((it) => !taken.includes(it)) ?? base;

export const editedLinkSuggestion = (editing: LinkEditing, host: string, port: string, portLabelText: string): string => {
  if (editing === LinkEditing.Serial) return autoSerialName(portLabelText);
  if (editing === LinkEditing.HostAndPort) return autoLinkName(LinkType.Tcp, host, port);
  return autoLinkName(LinkType.Udp, host, port);
};

export const DEFAULT_BAUD = 57600;

export type SerialPortChoice = { port: string; label: string };

const toIntOrNull = (text: string): number | null => {
  if (!/^[+-]?\d+$/.test(text)) return null;
  const value = Number.parseInt(text, 10);
  return value >= -2147483648 && value <= 2147483647 ? value : null;
};

const inPortRange = (port: number | null): port is number => port !== null && port >= 1 && port <= 65535;

const orIfBlank = (text: string, fallback: string): string => (text.trim() === '' ? fallback : text);

export const serialPortChoices = (view: Json | null): readonly SerialPortChoice[] =>
  jsonObjects(optJSONArray(view, 'serialPorts')).flatMap((entry) => {
    const port = optText(entry, 'port');
    return port.trim() === '' ? [] : [{ port, label: orIfBlank(optText(entry, 'label'), port) }];
  });

export const serialBauds = (view: Json | null): readonly number[] =>
  (optJSONArray(view, 'baudRates') ?? [])
    .map((rate) => (rate === null || rate === undefined ? null : toIntOrNull(String(rate))))
    .filter((it): it is number => it !== null && it > 0);

export const autoSerialName = (portLabelText: string): string => orIfBlank(portLabelText, 'Serial');

export const portLabel = (ports: readonly SerialPortChoice[], portName: string): string => ports.find((it) => it.port === portName)?.label ?? '';

export const portFor = (type: LinkType, udpDefault: string): string => (type === LinkType.Tcp ? DEFAULT_TCP_PORT : udpDefault);

export const serialFormError = (portName: string, baud: number, taken: readonly string[], name: string, anyPorts: boolean): string | null => {
  if (!anyPorts) return 'Nothing is plugged in. Connect a radio over USB and it will appear here.';
  if (portName.trim() === '') return 'Pick the port the radio is plugged into.';
  if (baud <= 0) return 'Pick a baud rate.';
  if (name.trim() !== '' && taken.includes(name.trim())) return 'A link with that name already exists.';
  return null;
};

export const udpServer = (typed: string, localPort: string): string | null => {
  const parts = typed.trim().split(':');
  const host = (parts[0] ?? '').trim();
  const port = toIntOrNull((parts[1] ?? localPort).trim());
  return parts.length > 2 || host === '' || !inPortRange(port) ? null : `${host}:${port}`;
};

export const withServer = (servers: readonly string[], typed: string, localPort: string): readonly string[] => {
  const server = udpServer(typed, localPort);
  return server === null || servers.includes(server) ? servers : [...servers, server];
};

const parsedPort = (type: LinkType, port: string, udpDefault: string): number | null => toIntOrNull(port.trim() === '' ? (type === LinkType.Udp ? udpDefault : port) : port);

export const linkFormError = (type: LinkType, host: string, port: string, udpDefault: string = DEFAULT_PORT): string | null => {
  const parsed = parsedPort(type, port, udpDefault);
  if (type === LinkType.Udp && !inPortRange(parsed)) return `Enter a port between 1 and 65535, or leave it blank for ${udpDefault}`;
  if (!inPortRange(parsed)) return 'Port must be a number between 1 and 65535.';
  if (type === LinkType.Tcp && host.trim() === '') return 'A TCP link needs the address of the device to call.';
  return null;
};

export const linkFormErrorField = (type: LinkType, host: string, port: string, udpDefault: string = DEFAULT_PORT): string | null => {
  if (!inPortRange(parsedPort(type, port, udpDefault))) return 'port';
  if (type === LinkType.Tcp && host.trim() === '') return 'host';
  return null;
};

const CONNECTED_STATUS = 'Connected';

const LinkIconBadge = ({ type, lit }: { type: LinkType; lit: boolean }) => (
  <span className="grid size-10 shrink-0 place-items-center rounded-full" style={{ background: lit ? aircast('successContainer') : md('secondaryContainer') }}>
    <Drawable name={linkIcon(type)} size={24} style={{ color: lit ? aircast('success') : md('onSecondaryContainer') }} />
  </span>
);

const AutoLinkItem = ({ link, onStop = null }: { link: AutoLink; onStop?: (() => void) | null }) => (
  <div className="flex min-h-[72px] w-full items-center gap-4 px-4 py-3">
    <LinkIconBadge type={link.type} lit={link.heard} />
    <div className="flex min-w-0 flex-1 flex-col">
      <Text typography="bodyLarge" maxLines={1} ellipsis>
        {link.name}
      </Text>
      <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
        {autoLinkSubtitle(link)}
      </Text>
    </div>
    <Text typography="labelMedium" color={link.heard ? aircast('success') : md('onSurfaceVariant')}>
      {autoLinkStatus(link)}
    </Text>
    {onStop !== null ? <TextButton onClick={onStop}>Stop</TextButton> : null}
  </div>
);

const LinkRowItem = ({ row, onConnect, onDisconnect, onRemove, onEdit }: { row: LinkRow; onConnect: () => void; onDisconnect: () => void; onRemove: () => void; onEdit: () => void }) => {
  const [menuOpen, setMenuOpen] = useState(false);
  const fixByEditing = row.errorRemedy === REMEDY_EDIT_ADDRESS && linkIsEditable(row);
  const advice = remedyText(row.errorRemedy);
  const primary = (
    <DropdownMenuItem
      key="primary"
      text={primaryLinkAction(row.connected, fixByEditing)}
      onClick={() => {
        setMenuOpen(false);
        if (row.connected) onDisconnect();
        else onConnect();
      }}
    />
  );
  const edit = (
    <DropdownMenuItem
      key="edit"
      text="Edit link"
      enabled={linkIsEditable(row)}
      onClick={() => {
        setMenuOpen(false);
        onEdit();
      }}
    />
  );
  return (
    <div className="relative w-full">
      <button
        type="button"
        aria-label={`More actions for ${row.name}`}
        onClick={() => setMenuOpen(true)}
        className="flex min-h-[72px] w-full items-center gap-4 px-4 py-3 text-left hover:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)]"
      >
        <LinkIconBadge type={row.type} lit={row.connected} />
        <span className="flex min-w-0 flex-1 flex-col">
          <Text typography="bodyLarge" maxLines={1} ellipsis>
            {row.name}
          </Text>
          {!(row.connected && row.statusLine === CONNECTED_STATUS) ? (
            <Text typography="bodyMedium" color={row.goneQuiet ? md('error') : md('onSurfaceVariant')}>
              {row.statusLine}
            </Text>
          ) : null}
          {row.lastError.trim() !== '' ? (
            <>
              <Text typography="bodySmall" color={md('error')}>
                {row.lastError}
              </Text>
              {advice !== null ? (
                <Text typography="bodySmall" color={md('onSurfaceVariant')}>
                  {advice}
                </Text>
              ) : null}
            </>
          ) : null}
        </span>
        {row.connected ? (
          <Text typography="labelLarge" color={aircast('success')}>
            {CONNECTED_STATUS}
          </Text>
        ) : null}
        <Drawable name="ic_chevron_right" style={{ color: md('onSurfaceVariant') }} />
      </button>
      <DropdownMenu expanded={menuOpen} onDismissRequest={() => setMenuOpen(false)}>
        {fixByEditing ? [edit, primary] : [primary, edit]}
        <DropdownMenuItem
          text="Delete link"
          onClick={() => {
            setMenuOpen(false);
            onRemove();
          }}
        />
      </DropdownMenu>
    </div>
  );
};

export const primaryLinkAction = (connected: boolean, fixByEditing: boolean): string => {
  if (connected) return 'Disconnect';
  if (fixByEditing) return 'Try again';
  return 'Connect';
};

const AdvancedDisclosure = ({ open, onToggle }: { open: boolean; onToggle: () => void }) => (
  <button type="button" aria-expanded={open} onClick={onToggle} className="flex min-h-12 w-full items-center text-left">
    <Text typography="titleSmall" className="flex-1">
      Advanced
    </Text>
    <MaterialIcon name={open ? 'keyboard_arrow_up' : 'keyboard_arrow_down'} filled label={open ? 'Hide advanced options' : 'Show advanced options'} />
  </button>
);

export const LINK_FLAG_TEXT: readonly (readonly [string, string])[] = [
  ['Connect on start', 'Open this link when the app launches.'],
  ['High latency', 'Tune the link for satellite or cellular round trips.'],
];

const LinkFlagSwitches = ({ autoConnect, highLatency, onAutoConnect, onHighLatency }: { autoConnect: boolean; highLatency: boolean; onAutoConnect: (checked: boolean) => void; onHighLatency: (checked: boolean) => void }) => {
  const states: readonly (readonly [boolean, (checked: boolean) => void])[] = [
    [autoConnect, onAutoConnect],
    [highLatency, onHighLatency],
  ];
  return (
    <>
      {LINK_FLAG_TEXT.map(([title, detail], at) => {
        const [checked, onChange] = states[at] ?? [false, () => undefined];
        return (
          <div key={title} className="flex w-full items-center">
            <div className="flex min-w-0 flex-1 flex-col">
              <Text>{title}</Text>
              <Text typography="bodySmall" color={md('onSurfaceVariant')}>
                {detail}
              </Text>
            </div>
            <Switch checked={checked} onCheckedChange={onChange} label={title} />
          </div>
        );
      })}
    </>
  );
};

export const linkFlagWrites = (index: number, autoConnect: boolean, highLatency: boolean): readonly (readonly [string, boolean])[] => [
  [`${LINKS_PATH}.${index}.autoConnect`, autoConnect],
  [`${LINKS_PATH}.${index}.highLatency`, highLatency],
];

export const framingWrites = (index: number, framing: SerialFraming): readonly (readonly [string, number])[] =>
  (
    [
      ['dataBits', framing.dataBits],
      ['stopBits', framing.stopBits],
      ['parity', framing.parity],
      ['flowControl', framing.flowControl],
    ] as const
  ).map(([field, value]) => [`${LINKS_PATH}.${index}.${field}`, value] as const);

const currentRows = async (): Promise<readonly LinkRow[]> => linkRows(await Qgc.get(LINKS_VIEW));

const rowNamed = async (name: string): Promise<LinkRow | null> => (await currentRows()).find((it) => it.name === name) ?? null;

const writeNewLinkFraming = async (name: string, framing: SerialFraming): Promise<boolean> => {
  if (same(framing, SerialFraming())) return true;
  const row = await rowNamed(name);
  if (row === null) return false;
  await Promise.all(framingWrites(row.index, framing).map(([path, value]) => Qgc.set(path, value)));
  await LinkCommands.commitConfigurations();
  return true;
};

const addServers = async (name: string, servers: readonly string[]): Promise<boolean> => {
  if (servers.length === 0) return true;
  const row = await rowNamed(name);
  if (row === null) return false;
  await Promise.all(servers.map((it) => Qgc.invoke(`${LINKS_PATH}.${row.index}.addHost`, it)));
  await LinkCommands.commitConfigurations();
  return true;
};

const writeNewLinkFlags = async (name: string, autoConnect: boolean, highLatency: boolean): Promise<void> => {
  if (!autoConnect && !highLatency) return;
  const row = await rowNamed(name);
  if (row === null) return;
  await Promise.all(linkFlagWrites(row.index, autoConnect, highLatency).map(([path, value]) => Qgc.set(path, value)));
  await LinkCommands.commitConfigurations();
};

const BLUETOOTH_POLL_MS = 1000;

const BluetoothPicker = ({ chosen, onPick }: { chosen: BluetoothDeviceChoice | null; onPick: (device: BluetoothDeviceChoice) => void }) => {
  const [state, setState] = useState<BluetoothState>({ available: false, scanning: false, devices: [] });
  useTicker(BLUETOOTH_POLL_MS, [], async () => setState(bluetoothState(await Qgc.get(LINKS_VIEW))));
  return (
    <div className="flex flex-col gap-1">
      <Text typography="bodySmall">{`Device: ${chosen?.name ?? ''}`}</Text>
      <Text typography="bodySmall">{`Address: ${chosen?.address ?? ''}`}</Text>
      <Text typography="titleSmall">Bluetooth devices</Text>
      {state.devices.map((device) => (
        <FilterChip key={device.address} selected={same(device, chosen)} onClick={() => onPick(device)} label={device.name} className="self-start" />
      ))}
      <div className="flex gap-2">
        <OutlinedButton enabled={!state.scanning} onClick={() => void LinkCommands.scanBluetooth(true)}>
          Scan
        </OutlinedButton>
        <OutlinedButton enabled={state.scanning} onClick={() => void LinkCommands.scanBluetooth(false)}>
          Stop
        </OutlinedButton>
      </div>
    </div>
  );
};

export type LinkEdit = {
  name: string;
  host: string;
  port: string;
  portName: string;
  baud: number;
  autoConnect: boolean;
  highLatency: boolean;
  framing: SerialFraming;
  logFile: string;
};

export const linkEdit = (row: LinkRow): LinkEdit => ({
  name: row.name,
  host: row.host,
  port: String(row.port),
  portName: row.portName,
  baud: row.baud > 0 ? row.baud : DEFAULT_BAUD,
  autoConnect: row.autoConnect,
  highLatency: row.highLatency,
  framing: row.framing,
  logFile: row.filename,
});

export const editShowsAdvanced = (row: LinkRow): boolean => row.autoConnect || row.highLatency || !same(row.framing, SerialFraming());

export const editedPort = (editing: LinkEditing, port: string, udpDefault: string): number =>
  toIntOrNull(port.trim() === '' ? (editing === LinkEditing.PortOnly ? udpDefault : port) : port) ?? 0;

export const editError = (editing: LinkEditing, edit: LinkEdit, udpDefault: string): string | null => {
  if (editing === LinkEditing.LogFile) return edit.logFile.trim() === '' ? 'Choose a log file.' : null;
  if (editing === LinkEditing.PortOnly) return linkFormError(LinkType.Udp, '', edit.port, udpDefault);
  if (editing === LinkEditing.HostAndPort) return inPortRange(editedPort(editing, edit.port, udpDefault)) ? null : 'Port must be a number between 1 and 65535.';
  return null;
};

export const editSuggestion = (row: LinkRow, edit: LinkEdit, device: BluetoothDeviceChoice | null, ports: readonly SerialPortChoice[]): string =>
  row.editing === LinkEditing.Device ? (device?.name ?? row.name) : editedLinkSuggestion(row.editing, edit.host, edit.port, portLabel(ports, edit.portName));

export const EDIT_WHILE_CONNECTED = 'Disconnect this link to change it.';

const applyEdit = async (index: number, writes: readonly LinkWrite[], device: BluetoothDeviceChoice | null): Promise<void> => {
  await Promise.all(writes.map(([field, value]) => Qgc.set(`${LINKS_PATH}.${index}.${field}`, value)));
  if (device !== null) await Qgc.invoke(`${LINKS_PATH}.${index}.setDeviceByAddress`, device.address);
  await LinkCommands.commitConfigurations();
  await LinkCommands.connect(`@${LINKS_PATH}.${index}`);
};

const LinkPortField = ({ label, value, error, enabled, onChange }: { label: string; value: string; error: string | null; enabled: boolean; onChange: (value: string) => void }) => (
  <OutlinedTextField value={value} onValueChange={onChange} label={label} enabled={enabled} isError={error !== null} supportingText={error ?? undefined} keyboardType="number" className="w-full" />
);

const LinkHostField = ({ value, error, enabled, onChange }: { value: string; error: string | null; enabled: boolean; onChange: (value: string) => void }) => (
  <OutlinedTextField
    value={value}
    onValueChange={onChange}
    label="Host address"
    placeholder="Example: 192.168.1.10"
    enabled={enabled}
    isError={error !== null}
    supportingText={error ?? undefined}
    keyboardType="uri"
    autoCorrect={false}
    className="w-full"
  />
);

const SerialPortFields = ({ ports, bauds, portName, baud, enabled, onPort, onBaud }: { ports: readonly SerialPortChoice[]; bauds: readonly number[]; portName: string; baud: number; enabled: boolean; onPort: (port: string) => void; onBaud: (baud: number) => void }) => (
  <>
    <ChoiceField
      label="USB port"
      value={orIfBlank(portLabel(ports, portName), orIfBlank(portName, 'Choose a port'))}
      options={ports.map((it) => it.label)}
      className="w-full"
      enabled={enabled}
      onPick={(it) => {
        const port = ports[it];
        if (port !== undefined) onPort(port.port);
      }}
    />
    <ChoiceField
      label="Baud rate"
      value={`${baud}`}
      options={bauds.map(String)}
      className="w-full"
      enabled={enabled}
      onPick={(it) => {
        const rate = bauds[it];
        if (rate !== undefined) onBaud(rate);
      }}
    />
  </>
);

const EditLinkFields = ({
  editing,
  edit,
  enabled,
  ports,
  bauds,
  device,
  onDevice,
  onPickLog,
  onEdit,
}: {
  editing: LinkEditing;
  edit: LinkEdit;
  enabled: boolean;
  ports: readonly SerialPortChoice[];
  bauds: readonly number[];
  device: BluetoothDeviceChoice | null;
  onDevice: (device: BluetoothDeviceChoice) => void;
  onPickLog: () => void;
  onEdit: (edit: LinkEdit) => void;
}) => {
  if (editing === LinkEditing.Device) return <BluetoothPicker chosen={device} onPick={onDevice} />;
  if (editing === LinkEditing.LogFile) {
    return (
      <>
        <Text typography="bodyLarge">{orIfBlank(leafName(edit.logFile), 'No log file chosen')}</Text>
        <OutlinedButton enabled={enabled} onClick={onPickLog} className="w-full desktop:w-auto desktop:self-start">
          Choose another log file
        </OutlinedButton>
      </>
    );
  }
  if (editing === LinkEditing.Serial) {
    return <SerialPortFields ports={ports} bauds={bauds} portName={edit.portName} baud={edit.baud} enabled={enabled} onPort={(it) => onEdit({ ...edit, portName: it })} onBaud={(it) => onEdit({ ...edit, baud: it })} />;
  }
  if (editing === LinkEditing.HostAndPort) {
    return (
      <>
        <LinkHostField value={edit.host} error={null} enabled={enabled} onChange={(it) => onEdit({ ...edit, host: it })} />
        <LinkPortField label="Port" value={edit.port} error={null} enabled={enabled} onChange={(it) => onEdit({ ...edit, port: it })} />
      </>
    );
  }
  return <LinkPortField label={editing === LinkEditing.PortOnly ? 'Listening port' : 'Port'} value={edit.port} error={null} enabled={enabled} onChange={(it) => onEdit({ ...edit, port: it })} />;
};

const udpDefaultOf = (udpListen: unknown): string =>
  typeof udpListen === 'number' && inPortRange(Math.trunc(udpListen)) ? String(Math.trunc(udpListen)) : DEFAULT_PORT;

const PageColumn = ({ title, backLabel, onBack, className, padded = true, children }: { title: string; backLabel: string; onBack: () => void; className?: string; padded?: boolean; children: ReactNode }) => {
  const heading = useContext(LocalPageHeading);
  return (
    <div className={`flex h-full w-full flex-col ${className ?? ''}`}>
      {heading === null ? <PageTopBar title={title} backLabel={backLabel} onBack={onBack} /> : null}
      <div className={`flex min-h-0 flex-1 flex-col overflow-auto ${padded ? 'gap-form px-page py-page' : ''}`}>{children}</div>
    </div>
  );
};

const EditLinkPage = ({ row, onDismiss, onSaved, className }: { row: LinkRow; onDismiss: () => void; onSaved: () => void; className?: string }) => {
  const [edit, setEdit] = useState(() => linkEdit(row));
  const [device, setDevice] = useState<BluetoothDeviceChoice | null>(null);
  const [advanced, setAdvanced] = useState(() => editShowsAdvanced(row));
  const [error, setError] = useState<string | null>(null);
  const cancel = () => {
    if (row.editing === LinkEditing.LogFile) void pruneReplayFolder(row.name, row.filename);
    onDismiss();
  };
  const pickLog = () =>
    void (async () => {
      const chosen = await openDocument(['*/*']);
      if (chosen === null) return;
      const staged = await stagedReplayLog(row.name, chosen);
      if (staged === null) setError('That file could not be read.');
      else setEdit((it) => ({ ...it, logFile: staged }));
    })();
  const linksJson = useQgcPath(LINKS_VIEW);
  const bauds = useMemo(() => {
    const served = serialBauds(linksJson);
    return served.length === 0 ? [DEFAULT_BAUD] : served;
  }, [linksJson]);
  const ports = useMemo(() => serialPortChoices(linksJson), [linksJson]);
  const udpDefault = udpDefaultOf(useQgcValue(UDP_LISTEN_PORT));
  const editable = linkRows(linksJson).find((it) => it.index === row.index)?.connected !== true;
  const about = linkKind(row.type).about;

  useBackHandler(true, cancel);
  const save = () => {
    const invalid = editError(row.editing, edit, udpDefault);
    setError(invalid);
    if (invalid !== null) return;
    void (async () => {
      const rows = await currentRows();
      if (rows.find((it) => it.index === row.index)?.connected !== false) {
        setError(EDIT_WHILE_CONNECTED);
        return;
      }
      const typedName = edit.name.trim();
      const resolved = typedName === '' ? uniqueLinkName(editSuggestion(row, edit, device, ports), rows.filter((it) => it.index !== row.index).map((it) => it.name)) : typedName;
      const writes = editWrites(row.editing, resolved, edit.host, editedPort(row.editing, edit.port, udpDefault), edit.portName, edit.baud, edit.autoConnect, edit.highLatency, edit.framing, edit.logFile);
      await applyEdit(row.index, writes, device);
      if (row.editing === LinkEditing.LogFile) await pruneReplayFolder(row.name, edit.logFile);
      onSaved();
    })();
  };

  return (
    <>
      <OverridePageHeading title={`Edit ${row.name}`} onBack={cancel} />
      <PageColumn title={`Edit ${row.name}`} backLabel="Back to links" onBack={cancel} className={className}>
        {about.trim() !== '' ? (
          <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
            {about}
          </Text>
        ) : null}
        {!editable ? (
          <Text typography="bodyMedium" color={md('error')}>
            {EDIT_WHILE_CONNECTED}
          </Text>
        ) : null}
        <EditLinkFields editing={row.editing} edit={edit} enabled={editable} ports={ports} bauds={bauds} device={device} onDevice={setDevice} onPickLog={pickLog} onEdit={setEdit} />
        {row.editing === LinkEditing.PortOnly ? <UdpServers index={row.index} initial={row.servers} /> : null}
        <AdvancedDisclosure open={advanced} onToggle={() => setAdvanced(!advanced)} />
        {advanced ? (
          <>
            <OutlinedTextField value={edit.name} onValueChange={(it) => setEdit({ ...edit, name: it })} label="Name" enabled={editable} className="w-full" />
            {row.editing === LinkEditing.Serial ? <SerialFramingControls framing={edit.framing} onChange={(it) => setEdit({ ...edit, framing: it })} /> : null}
            <LinkFlagSwitches autoConnect={edit.autoConnect} highLatency={edit.highLatency} onAutoConnect={(it) => setEdit({ ...edit, autoConnect: it })} onHighLatency={(it) => setEdit({ ...edit, highLatency: it })} />
          </>
        ) : null}
        {error !== null ? (
          <Text typography="bodySmall" color={md('error')}>
            {error}
          </Text>
        ) : null}
        <Button enabled={editable} onClick={save} className="min-h-[52px] w-full desktop:w-auto desktop:min-w-40 desktop:self-end">
          Save and connect
        </Button>
      </PageColumn>
    </>
  );
};

export type LinkKind = { type: LinkType; title: string; detail: string; about: string };

export const PILOT_LINK_ORDER: readonly LinkType[] = [LinkType.WebSocket, LinkType.Udp, LinkType.Serial, LinkType.Bluetooth, LinkType.AircastCloud, LinkType.Tcp];
export const TOOL_LINK_ORDER: readonly LinkType[] = [LinkType.LogReplay, LinkType.Mock];

const LINK_KINDS: Record<Exclude<LinkType['name'], 'Other'>, readonly [string, string, string]> = {
  Udp: ['Wi-Fi or network', 'Wi-Fi or Ethernet (UDP)', 'Listens on a port for telemetry from the aircraft or its radio.'],
  Serial: ['USB telemetry radio', 'MAVLink radio on USB', 'A telemetry radio or flight controller plugged into this device over USB.'],
  Bluetooth: ['Bluetooth radio', 'Paired telemetry radio', 'A radio paired with or near this device over Bluetooth.'],
  AircastCloud: ['Aircast Cloud', 'Backup link via your account', 'A backup link to the aircraft through your Aircast account.'],
  Tcp: ['Network server (TCP)', 'A listening ground station', 'Calls out to a device that is listening, such as a ground station or bridge.'],
  LogReplay: ['Replay a flight log', 'Play back a saved log', 'Plays back a saved telemetry log as if the vehicle were connected.'],
  Mock: ['Simulated vehicle', 'Try the app without hardware', 'A simulated vehicle for trying the app without hardware. It is not saved, so it ends when the app restarts.'],
  WebSocket: ['Network (WebSocket)', 'MAVLink over WebSocket', 'A browser cannot open UDP or TCP sockets, so it reaches the aircraft through a MAVLink WebSocket, such as a bridge on the ground station or the aircraft. The link is saved and reconnects when the page opens again.'],
};

export const linkKind = (type: LinkType): LinkKind => {
  if (type.name === 'Other') return { type, title: linkTypeLabel(type), detail: '', about: '' };
  const [title, detail, about] = LINK_KINDS[type.name];
  return { type, title, detail, about };
};

export const pilotLinkKinds = (offered: readonly LinkType[], radioPlugged: boolean): readonly LinkType[] =>
  PILOT_LINK_ORDER.filter((it) => offered.includes(it)).toSorted((a, b) => (radioPlugged && a === LinkType.Serial ? 0 : 1) - (radioPlugged && b === LinkType.Serial ? 0 : 1));

export const toolLinkKinds = (offered: readonly LinkType[]): readonly LinkType[] => TOOL_LINK_ORDER.filter((it) => offered.includes(it));

export type LinkDraft = {
  type: LinkType;
  name: string;
  host: string;
  port: string;
  servers: readonly string[];
  portName: string;
  baud: number;
  framing: SerialFraming;
  autoConnect: boolean;
  highLatency: boolean;
  device: BluetoothDeviceChoice | null;
  replayLog: string;
  apiBase: string;
  deviceId: string;
  mock: MockLinkChoices;
};

export const LinkDraft = (fields: Pick<LinkDraft, 'type'> & Partial<LinkDraft>): LinkDraft => ({
  name: '',
  host: '',
  port: '',
  servers: [],
  portName: '',
  baud: DEFAULT_BAUD,
  framing: SerialFraming(),
  autoConnect: false,
  highLatency: false,
  device: null,
  replayLog: '',
  apiBase: DEFAULT_ACCOUNT_SERVER,
  deviceId: '',
  mock: MockLinkChoices(),
  ...fields,
});

export const shownPort = (draft: LinkDraft, udpDefault: string): string => orIfBlank(draft.port, portFor(draft.type, udpDefault));

export const withPluggedPort = (draft: LinkDraft, ports: readonly SerialPortChoice[]): LinkDraft => ({
  ...draft,
  portName: orIfBlank(draft.portName, ports.length === 1 ? (ports[0]?.port ?? '') : ''),
});

export const suggestedLinkName = (draft: LinkDraft, ports: readonly SerialPortChoice[], udpDefault: string): string => {
  if (draft.type === LinkType.Serial) return autoSerialName(portLabel(ports, draft.portName));
  if (draft.type === LinkType.LogReplay) return REPLAY_LINK_NAME;
  if (draft.type === LinkType.AircastCloud) return AIRCAST_CLOUD_NAME;
  if (draft.type === LinkType.Bluetooth) return draft.device?.name ?? '';
  return autoLinkName(draft.type, draft.host, shownPort(draft, udpDefault));
};

export const draftError = (draft: LinkDraft, udpDefault: string, taken: readonly string[], anyPorts: boolean): string | null => {
  if (draft.type === LinkType.Mock) return null;
  if (draft.type === LinkType.AircastCloud) return cloudApiBaseValid(draft.apiBase) && cloudDeviceValid(draft.deviceId) ? null : '';
  if (draft.type === LinkType.Bluetooth) return draft.device === null ? 'Pick a Bluetooth device.' : null;
  if (draft.type === LinkType.LogReplay) return draft.replayLog.trim() === '' ? 'Choose a log file to replay.' : null;
  if (draft.type === LinkType.Serial) return serialFormError(draft.portName, draft.baud, taken, draft.name, anyPorts);
  return linkFormError(draft.type, draft.host, shownPort(draft, udpDefault), udpDefault);
};

export type AddOutcome = { readonly name: 'Connected' | 'SavedNotConnected' | 'Failed'; readonly ordinal: number };

export const AddOutcome = {
  Connected: { name: 'Connected', ordinal: 0 },
  SavedNotConnected: { name: 'SavedNotConnected', ordinal: 1 },
  Failed: { name: 'Failed', ordinal: 2 },
} as const satisfies Record<string, AddOutcome>;

export const AddOutcomeEntries: readonly AddOutcome[] = [AddOutcome.Connected, AddOutcome.SavedNotConnected, AddOutcome.Failed];

export const addOutcome = (created: boolean, saved: boolean): AddOutcome => (created ? AddOutcome.Connected : saved ? AddOutcome.SavedNotConnected : AddOutcome.Failed);

export const addFailure = (type: LinkType): string => (type === LinkType.Mock ? 'Could not start the simulated vehicle.' : 'Could not add that link. The name may already be in use.');

const createCloudLink = async (draft: LinkDraft, name: string): Promise<boolean> => {
  await AccountCommands.setApiBase(draft.apiBase);
  return LinkCommands.createAircastCloud(name, draft.apiBase, draft.deviceId);
};

const createLink = async (draft: LinkDraft, name: string, port: number, staged: string | null): Promise<boolean> => {
  if (draft.type === LinkType.Mock) return LinkCommands.startMock([...mockLinkArguments(draft.mock)]);
  if (draft.type === LinkType.AircastCloud) return createCloudLink(draft, name);
  if (draft.type === LinkType.LogReplay) return staged !== null && (await LinkCommands.createLogReplay(name, staged)) && connectNamed(name);
  if (draft.type === LinkType.Bluetooth) return draft.device !== null && LinkCommands.createBluetooth(name, draft.device.name, draft.device.address);
  if (draft.type === LinkType.Serial) return (await LinkCommands.createSerial(name, draft.portName, draft.baud)) && (await writeNewLinkFraming(name, draft.framing)) && connectNamed(name);
  return (await Qgc.invokeResult('links.createAndConnectLink', draft.type.id, name, draft.type === LinkType.Udp ? '' : draft.host, port)) === true;
};

const addLink = async (draft: LinkDraft, name: string, udpDefault: string, replayFile: File | null): Promise<AddOutcome> => {
  const staged = draft.type === LinkType.LogReplay && replayFile !== null ? await stagedReplayLog(name, replayFile) : null;
  const created = await createLink(draft, name, toIntOrNull(shownPort(draft, udpDefault)) ?? 0, staged);
  const saved = draft.type !== LinkType.Mock && (await currentRows()).some((it) => it.name === name);
  const outcome = addOutcome(created, saved);
  if (outcome === AddOutcome.Failed && staged !== null) await discardStagedLog(staged);
  if (saved) {
    await addServers(name, draft.servers);
    await writeNewLinkFlags(name, draft.autoConnect, draft.highLatency);
  }
  return outcome;
};

const AddLinkPage = ({ onDismiss, onAdded, className }: { onDismiss: () => void; onAdded: () => void; className?: string }) => {
  const [type, setType] = useState<LinkType | null>(null);
  const [transmitter, setTransmitter] = useState(false);
  const linksJson = useQgcPath(LINKS_VIEW);
  const ports = useMemo(() => serialPortChoices(linksJson), [linksJson]);
  const offered = useMemo(() => addableLinkTypes(linksJson), [linksJson]);
  const [cloudOffered, setCloudOffered] = useState(false);
  useLaunchedEffect([], async (scope) => {
    const offeredNow = accountState(await AccountCommands.state()) !== null;
    if (scope.isActive()) setCloudOffered(offeredNow);
  });
  const choices = [...offered, ...[LinkType.AircastCloud].filter((it) => cloudOffered && linkTypeIds(linksJson).has(it))];
  useBackHandler(type === null && !transmitter, onDismiss);

  if (transmitter) return <RcTransmitterPage onBack={() => setTransmitter(false)} onDone={onAdded} className={className} />;
  if (type === LinkType.WebSocket) return <WebSocketLinkPage onBack={() => setType(null)} onAdded={onAdded} className={className} />;
  if (type !== null) return <AddLinkDetails type={type} onBack={() => setType(null)} onAdded={onAdded} className={className} />;

  const row = (kind: LinkKind) => <SetupRow key={kind.type.name} title={kind.title} subtitle={kind.detail} icon={linkIcon(kind.type)} onClick={() => setType(kind.type)} />;
  const tools = toolLinkKinds(choices);
  return (
    <>
      <OverridePageHeading title="Add link" onBack={onDismiss} />
      <PageColumn title="Add link" backLabel="Back to links" onBack={onDismiss} className={className} padded={false}>
        <SectionHeader text="Connect over" />
        {pilotLinkKinds(choices, ports.length > 0).map(linkKind).map(row)}
        {choices.includes(LinkType.Serial) ? <SetupRow title={RC_TRANSMITTER.title} subtitle={RC_TRANSMITTER.detail} icon={linkIcon(LinkType.Serial)} onClick={() => setTransmitter(true)} /> : null}
        {tools.length > 0 ? (
          <>
            <SectionHeader text="Other" />
            {tools.map(linkKind).map(row)}
          </>
        ) : null}
      </PageColumn>
    </>
  );
};

export const RC_TRANSMITTER = {
  title: 'RC transmitter (EdgeTX script)',
  detail: 'Your radio’s sticks over USB',
  about: 'An EdgeTX radio running the Aircast USB script sends its sticks to this app. Pick the port it is plugged into.',
  picked: 'The radio sends its sticks to this app. Its screen shows whether the app hears it. Change the port under Control > Radio on USB.',
};

const RcTransmitterPage = ({ onBack, onDone, className }: { onBack: () => void; onDone: () => void; className?: string }) => {
  const ports = serialPortChoices(useQgcPath(LINKS_VIEW));
  const current = String(useQgcValue(USB_RADIO_PORT) ?? '');
  const [picked, setPicked] = useState(false);
  useBackHandler(true, onBack);
  const pick = (port: string) =>
    void (async () => {
      setPicked(await Qgc.set(`${USB_RADIO_PORT}.rawValue`, port));
    })();
  return (
    <>
      <OverridePageHeading title={RC_TRANSMITTER.title} onBack={onBack} />
      <PageColumn title={RC_TRANSMITTER.title} backLabel="Back to link types" onBack={onBack} className={className}>
        <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
          {RC_TRANSMITTER.about}
        </Text>
        {ports.length === 0 && serialAvailable() ? (
          <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
            Nothing is plugged in. Connect a radio over USB and it will appear here.
          </Text>
        ) : null}
        {ports.map((it) => (
          <SetupRow key={it.port} title={it.label} subtitle={it.port === current ? 'Sends the sticks' : ''} icon={linkIcon(LinkType.Serial)} onClick={() => pick(it.port)} />
        ))}
        <UsbAccess />
        {picked ? (
          <>
            <Text typography="bodyMedium">{RC_TRANSMITTER.picked}</Text>
            <Button onClick={onDone} className="min-h-[52px] w-full desktop:w-auto desktop:min-w-40 desktop:self-end">
              Done
            </Button>
          </>
        ) : null}
      </PageColumn>
    </>
  );
};

const NewLinkFields = ({
  draft,
  ports,
  bauds,
  hostError,
  portError,
  cloudErrors,
  onPickLog,
  onDraft,
}: {
  draft: LinkDraft;
  ports: readonly SerialPortChoice[];
  bauds: readonly number[];
  hostError: string | null;
  portError: string | null;
  cloudErrors: boolean;
  onPickLog: () => void;
  onDraft: (draft: LinkDraft) => void;
}) => {
  if (draft.type === LinkType.Mock) return <MockLinkFields choices={draft.mock} onChange={(it) => onDraft({ ...draft, mock: it })} />;
  if (draft.type === LinkType.LogReplay) {
    return (
      <OutlinedButton onClick={onPickLog} className="w-full desktop:w-auto desktop:self-start">
        {draft.replayLog.trim() === '' ? 'Choose a log file' : leafName(draft.replayLog)}
      </OutlinedButton>
    );
  }
  if (draft.type === LinkType.AircastCloud) {
    return <AircastCloudFields apiBase={draft.apiBase} deviceId={draft.deviceId} showErrors={cloudErrors} onApiBase={(it) => onDraft({ ...draft, apiBase: it })} onDeviceId={(it) => onDraft({ ...draft, deviceId: it })} />;
  }
  if (draft.type === LinkType.Bluetooth) return <BluetoothPicker chosen={draft.device} onPick={(it) => onDraft({ ...draft, device: it })} />;
  if (draft.type === LinkType.Serial) {
    return (
      <>
        {ports.length === 0 ? (
          serialAvailable() ? (
            <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
              Nothing is plugged in. Connect a radio over USB and it will appear here.
            </Text>
          ) : null
        ) : (
          <SerialPortFields ports={ports} bauds={bauds} portName={draft.portName} baud={draft.baud} enabled onPort={(it) => onDraft({ ...draft, portName: it })} onBaud={(it) => onDraft({ ...draft, baud: it })} />
        )}
        <UsbAccess />
      </>
    );
  }
  if (draft.type === LinkType.Udp) return <LinkPortField label="Listening port" value={draft.port} error={portError} enabled onChange={(it) => onDraft({ ...draft, port: it })} />;
  return (
    <>
      <LinkHostField value={draft.host} error={hostError} enabled onChange={(it) => onDraft({ ...draft, host: it })} />
      <LinkPortField label="Port" value={draft.port} error={portError} enabled onChange={(it) => onDraft({ ...draft, port: it })} />
    </>
  );
};

const DraftServers = ({ draft, udpDefault, onError, onDraft }: { draft: LinkDraft; udpDefault: string; onError: (error: string | null) => void; onDraft: (draft: LinkDraft) => void }) => (
  <ServerList
    servers={draft.servers}
    onAdd={(typed) => {
      onError(udpServer(typed, shownPort(draft, udpDefault)) === null ? 'Enter a server as an address, or address:port.' : null);
      onDraft({ ...draft, servers: withServer(draft.servers, typed, shownPort(draft, udpDefault)) });
    }}
    onRemove={(gone) => onDraft({ ...draft, servers: draft.servers.filter((it) => it !== gone) })}
  />
);

const NewLinkAdvanced = ({ draft, suggestedName, ports, onDraft }: { draft: LinkDraft; suggestedName: string; ports: readonly SerialPortChoice[]; onDraft: (draft: LinkDraft) => void }) => (
  <>
    <OutlinedTextField value={draft.name} onValueChange={(it) => onDraft({ ...draft, name: it })} label="Name" placeholder={suggestedName} className="w-full" />
    {draft.type === LinkType.Serial && ports.length > 0 ? <SerialFramingControls framing={draft.framing} onChange={(it) => onDraft({ ...draft, framing: it })} /> : null}
    <LinkFlagSwitches autoConnect={draft.autoConnect} highLatency={draft.highLatency} onAutoConnect={(it) => onDraft({ ...draft, autoConnect: it })} onHighLatency={(it) => onDraft({ ...draft, highLatency: it })} />
  </>
);

export const DEFAULT_WEBSOCKET_URL = 'wss://sitl.aircast.one/ws/mavlink';

export const WEBSOCKET_NOT_SAVED = 'The WebSocket link could not be saved.';

export const webSocketLinkName = (typed: string): string => {
  try {
    return new URL(typed.trim()).host || 'WebSocket';
  } catch {
    return 'WebSocket';
  }
};

export const webSocketUrlError = (typed: string): string | null => {
  try {
    const url = new URL(typed.trim());
    return url.protocol === 'ws:' || url.protocol === 'wss:' ? null : 'A MAVLink WebSocket address starts with ws:// or wss://.';
  } catch {
    return 'Enter an address such as ws://192.168.1.10:5760.';
  }
};

const WebSocketLinkPage = ({ onBack, onAdded, className }: { onBack: () => void; onAdded: () => void; className?: string }) => {
  const [typed, setTyped] = useState(DEFAULT_WEBSOCKET_URL);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const kind = linkKind(LinkType.WebSocket);
  const linksJson = useQgcPath(LINKS_VIEW);
  const taken = useMemo(() => linkRows(linksJson).map((it) => it.name), [linksJson]);
  useBackHandler(true, onBack);
  const connect = () => {
    const invalid = webSocketUrlError(typed);
    setError(invalid);
    if (invalid !== null) return;
    setBusy(true);
    void LinkCommands.createWebSocket(uniqueLinkName(webSocketLinkName(typed), taken), typed.trim()).then((saved) => {
      setBusy(false);
      if (saved) onAdded();
      else setError(WEBSOCKET_NOT_SAVED);
    });
  };
  return (
    <>
      <OverridePageHeading title={kind.title} onBack={onBack} />
      <PageColumn title={kind.title} backLabel="Back to link types" onBack={onBack} className={className}>
        <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
          {kind.about}
        </Text>
        <OutlinedTextField value={typed} onValueChange={setTyped} label="WebSocket address" keyboardType="uri" autoCorrect={false} isError={error !== null} supportingText={error ?? undefined} onDone={connect} singleLine />
        <div className="flex justify-end">
          <Button onClick={connect} enabled={!busy}>
            Connect
          </Button>
        </div>
      </PageColumn>
    </>
  );
};

const AddLinkDetails = ({ type, onBack, onAdded, className }: { type: LinkType; onBack: () => void; onAdded: () => void; className?: string }) => {
  const linksJson = useQgcPath(LINKS_VIEW);
  const ports = useMemo(() => serialPortChoices(linksJson), [linksJson]);
  const bauds = useMemo(() => {
    const served = serialBauds(linksJson);
    return served.length === 0 ? [DEFAULT_BAUD] : served;
  }, [linksJson]);
  const taken = useMemo(() => linkRows(linksJson).map((it) => it.name), [linksJson]);
  const udpDefault = udpDefaultOf(useQgcValue(UDP_LISTEN_PORT));
  const [typed, setTyped] = useState(() => LinkDraft({ type, port: portFor(type, udpDefault), mock: preferredMockChoices(linksJson) }));
  const [replayFile, setReplayFile] = useState<File | null>(null);
  const draft = withPluggedPort(typed, ports);
  const [advanced, setAdvanced] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [cloudErrors, setCloudErrors] = useState(false);
  const pickLog = () =>
    void (async () => {
      const chosen = await openDocument(['*/*']);
      if (chosen === null) return;
      setReplayFile(chosen);
      setTyped((it) => ({ ...it, replayLog: chosen.name }));
    })();
  const formTyped = type === LinkType.Udp || type === LinkType.Tcp;
  const fieldError =
    error !== null && formTyped && error === linkFormError(type, draft.host, shownPort(draft, udpDefault), udpDefault) ? linkFormErrorField(type, draft.host, shownPort(draft, udpDefault), udpDefault) : null;
  const suggestedName = suggestedLinkName(draft, ports, udpDefault);
  const change = (changed: LinkDraft) => {
    setTyped(changed);
    if (fieldError !== null) setError(null);
  };
  const title = linkKind(type).title;

  useBackHandler(true, onBack);
  const connect = () => {
    setCloudErrors(type === LinkType.AircastCloud);
    const invalid = draftError(draft, udpDefault, taken, ports.length > 0);
    setError(invalid === null || invalid.trim() === '' ? null : invalid);
    if (invalid !== null) return;
    setBusy(true);
    const typedName = draft.name.trim();
    const name = typedName === '' ? uniqueLinkName(suggestedName, taken) : typedName;
    void (async () => {
      const outcome = await addLink(draft, name, udpDefault, replayFile);
      setBusy(false);
      if (outcome === AddOutcome.Failed) setError(addFailure(type));
      else onAdded();
    })();
  };

  return (
    <>
      <OverridePageHeading title={title} onBack={onBack} />
      <PageColumn title={title} backLabel="Back to link types" onBack={onBack} className={className}>
        <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
          {linkKind(type).about}
        </Text>
        <NewLinkFields
          draft={draft}
          ports={ports}
          bauds={bauds}
          hostError={fieldError === 'host' ? error : null}
          portError={fieldError === 'port' ? error : null}
          cloudErrors={cloudErrors}
          onPickLog={pickLog}
          onDraft={change}
        />
        {type === LinkType.Udp ? <DraftServers draft={draft} udpDefault={udpDefault} onError={setError} onDraft={change} /> : null}
        {type !== LinkType.Mock ? (
          <>
            <AdvancedDisclosure open={advanced} onToggle={() => setAdvanced(!advanced)} />
            {advanced ? <NewLinkAdvanced draft={draft} suggestedName={suggestedName} ports={ports} onDraft={change} /> : null}
          </>
        ) : null}
        {fieldError === null && error !== null ? (
          <Text typography="bodySmall" color={md('error')}>
            {error}
          </Text>
        ) : null}
        <Button enabled={!busy && (type !== LinkType.Serial || serialAvailable())} onClick={connect} className="min-h-[52px] w-full desktop:w-auto desktop:min-w-40 desktop:self-end">
          {busy ? 'Connecting…' : type === LinkType.Mock ? 'Start' : 'Connect'}
        </Button>
      </PageColumn>
    </>
  );
};

const LINK_SETTLE_MS = 4000;
const SETTLE_POLL_MS = 150;

export const linkFailure = (action: string, done: boolean): string | null => (done ? null : `Could not ${action} that link.`);

export const rowConnected = (rows: readonly LinkRow[], index: number): boolean | null => rows.find((it) => it.index === index)?.connected ?? null;

const connectNamed = async (name: string): Promise<boolean> => {
  const row = await rowNamed(name);
  return row !== null && (await LinkCommands.connect(`@${LINKS_PATH}.${row.index}`)) === null;
};

const settledWithin = async (deadline: number, settled: () => Promise<boolean>): Promise<boolean> => {
  if (Date.now() >= deadline) return false;
  if (await settled()) return true;
  await delay(SETTLE_POLL_MS);
  return settledWithin(deadline, settled);
};

export const LinksScreen = ({ className, footer = null }: { className?: string; footer?: ReactNode }) => {
  const view = useQgcPath(LINKS_VIEW);
  const hasVehicle = useHasVehicle();
  const rows = linkRows(view);
  const auto = autoLinks(view);
  const navigation = useContext(LocalAppNavigation);
  const addLinkRequested = useValue(navigation.addLinkRequested);
  const [notice, setNotice] = useState<string | null>(null);
  const [adding, setAdding] = useState(() => navigation.addLinkRequested.value);
  const [advanced, setAdvanced] = useState(false);
  const [confirmingDisconnect, setConfirmingDisconnect] = useState<LinkRow | null>(null);
  const [confirmingRemove, setConfirmingRemove] = useState<LinkRow | null>(null);
  const [editing, setEditing] = useState<LinkRow | null>(null);

  useEffect(() => {
    if (!addLinkRequested) return;
    setAdding(true);
    navigation.addLinkRequested.value = false;
  }, [addLinkRequested]);

  const attempt = (action: string, settled: () => Promise<boolean>, call: () => Promise<string | null>) =>
    void (async () => {
      const refused = await call();
      const done = refused === null && (await settledWithin(Date.now() + LINK_SETTLE_MS, settled));
      setNotice(refused ?? linkFailure(action, done));
    })();

  const disconnect = (row: LinkRow) =>
    attempt('disconnect', async () => rowConnected(await currentRows(), row.index) !== true, () => LinkCommands.disconnect(row.index));

  if (adding) return <AddLinkPage onDismiss={() => setAdding(false)} onAdded={() => setAdding(false)} className={className} />;
  if (editing !== null) return <EditLinkPage row={editing} onDismiss={() => setEditing(null)} onSaved={() => setEditing(null)} className={className} />;

  const groups: readonly (readonly [string, readonly LinkRow[]])[] = [
    ['Connected', rows.filter((it) => it.connected)],
    ['Saved', rows.filter((it) => !it.connected)],
  ];

  return (
    <div className={`relative flex h-full w-full flex-col ${className ?? ''}`}>
      {notice !== null ? (
        <Text typography="bodySmall" color={md('error')} className="block px-5 py-2">
          {notice}
        </Text>
      ) : null}
      <div className="flex min-h-0 flex-1 flex-col overflow-auto">
        {rows.length === 0 && auto.length === 0 ? <FootNote text="No links saved. A USB cable or telemetry radio connects by itself; add a link for Wi-Fi or a network connection." /> : null}
        {groups.map(([title, group]) => (
          <div key={`head${title}`} className="flex flex-col">
            {group.length > 0 || (title === 'Connected' && auto.length > 0) ? <SectionHeader text={title} /> : null}
            {group.map((row) => (
              <LinkRowItem
                key={`link${row.name}`}
                row={row}
                onConnect={() => attempt('connect', async () => rowConnected(await currentRows(), row.index) === true, () => LinkCommands.connect(`@${LINKS_PATH}.${row.index}`))}
                onDisconnect={() => (hasVehicle ? setConfirmingDisconnect(row) : disconnect(row))}
                onRemove={() => setConfirmingRemove(row)}
                onEdit={() => setEditing(row)}
              />
            ))}
            {title === 'Connected'
              ? auto.map((link) => (
                  <AutoLinkItem
                    key={`auto${link.name}`}
                    link={link}
                    onStop={
                      link.type === LinkType.Mock
                        ? () => attempt('stop', async () => autoLinks(await Qgc.get(LINKS_VIEW)).every((it) => it.name !== link.name), () => LinkCommands.disconnect(link.index))
                        : null
                    }
                  />
                ))
              : null}
          </div>
        ))}
        <Button onClick={() => setAdding(true)} className="mx-5 my-2 self-start">
          <Drawable name="ic_add" />
          <span className="ps-2">Add link…</span>
        </Button>
        <AdvancedToggle open={advanced} onToggle={() => setAdvanced(!advanced)} className="self-start" />
        {advanced ? footer : null}
      </div>

      {confirmingDisconnect !== null ? (
        <AlertDialog
          onDismissRequest={() => setConfirmingDisconnect(null)}
          title={`Disconnect ${confirmingDisconnect.name}?`}
          text="A vehicle is connected on this link. Disconnecting stops telemetry and you will not be able to command it until it reconnects."
          confirmButton={
            <Button
              onClick={() => {
                disconnect(confirmingDisconnect);
                setConfirmingDisconnect(null);
              }}
            >
              Disconnect
            </Button>
          }
          dismissButton={<TextButton onClick={() => setConfirmingDisconnect(null)}>Keep connected</TextButton>}
        />
      ) : null}

      {confirmingRemove !== null ? (
        <AlertDialog
          onDismissRequest={() => setConfirmingRemove(null)}
          title={`Remove ${confirmingRemove.name}?`}
          text={confirmingRemove.connected ? 'This link is connected. Removing it disconnects it first and forgets the settings.' : 'Aircast will forget this link and its settings.'}
          confirmButton={
            <Button
              onClick={() => {
                const row = confirmingRemove;
                attempt('remove', async () => (await currentRows()).every((it) => it.name !== row.name), () => LinkCommands.remove(`@${LINKS_PATH}.${row.index}`, row.name));
                setConfirmingRemove(null);
              }}
            >
              Remove
            </Button>
          }
          dismissButton={<TextButton onClick={() => setConfirmingRemove(null)}>Cancel</TextButton>}
        />
      ) : null}
    </div>
  );
};

export const REMEDY_EDIT_ADDRESS = 'editAddress';

export const remedyText = (remedy: string): string | null => (remedy === REMEDY_EDIT_ADDRESS ? 'Nothing is listening at that address. Retrying will not help until it is changed.' : null);

const SerialFramingControls = ({ framing, onChange }: { framing: SerialFraming; onChange: (framing: SerialFraming) => void }) => (
  <div className="flex flex-col">
    <div className="flex items-center gap-3">
      <Checkbox checked={framing.flowControl !== 0} onCheckedChange={(it) => onChange({ ...framing, flowControl: it ? 1 : 0 })} label="Enable flow control" />
      <Text>Enable flow control</Text>
    </div>
    <FramingPicker
      label="Parity"
      shown={PARITY_CHOICES.find((it) => it[1] === framing.parity)?.[0] ?? ''}
      choices={PARITY_CHOICES.map((it) => it[0])}
      onPick={(it) => onChange({ ...framing, parity: PARITY_CHOICES[it]?.[1] ?? framing.parity })}
    />
    <FramingPicker
      label="Data Bits"
      shown={String(framing.dataBits)}
      choices={DATA_BITS_CHOICES.map(String)}
      onPick={(it) => onChange({ ...framing, dataBits: DATA_BITS_CHOICES[it] ?? framing.dataBits })}
    />
    <FramingPicker
      label="Stop Bits"
      shown={String(framing.stopBits)}
      choices={STOP_BITS_CHOICES.map(String)}
      onPick={(it) => onChange({ ...framing, stopBits: STOP_BITS_CHOICES[it] ?? framing.stopBits })}
    />
  </div>
);

const FramingPicker = ({ label, shown, choices, onPick }: { label: string; shown: string; choices: readonly string[]; onPick: (index: number) => void }) => {
  const [open, setOpen] = useState(false);
  return (
    <div className="flex items-center">
      <Text className="flex-1">{label}</Text>
      <div className="relative">
        <OutlinedButton onClick={() => setOpen(true)}>{shown}</OutlinedButton>
        <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
          {choices.map((choice, index) => (
            <DropdownMenuItem
              key={choice}
              text={choice}
              onClick={() => {
                onPick(index);
                setOpen(false);
              }}
            />
          ))}
        </DropdownMenu>
      </div>
    </div>
  );
};

const UdpServers = ({ index, initial }: { index: number; initial: readonly string[] }) => {
  const [servers, setServers] = useState(initial);
  const change = (action: string, host: string) =>
    void (async () => {
      await Qgc.invoke(`${LINKS_PATH}.${index}.${action}`, host);
      const served = (await currentRows()).find((it) => it.index === index)?.servers ?? null;
      setServers((it) => served ?? it);
    })();
  return <ServerList servers={servers} onAdd={(it) => change('addHost', it)} onRemove={(it) => change('removeHost', it)} />;
};

const ServerList = ({ servers, onAdd, onRemove }: { servers: readonly string[]; onAdd: (server: string) => void; onRemove: (server: string) => void }) => {
  const [typed, setTyped] = useState('');
  const add = () => {
    onAdd(typed.trim());
    setTyped('');
  };
  return (
    <div className="flex flex-col gap-1">
      {servers.map((server) => (
        <div key={server} className="flex min-h-12 w-full items-center">
          <Text className="flex-1">{server}</Text>
          <IconButton onClick={() => onRemove(server)} label={`Remove ${server}`}>
            <Drawable name="ic_close" />
          </IconButton>
        </div>
      ))}
      <OutlinedTextField
        value={typed}
        onValueChange={setTyped}
        label="Also send to"
        placeholder="127.0.0.1:14550"
        supportingText="Optional. One address per device that should get telemetry too."
        keyboardType="uri"
        onDone={() => {
          if (typed.trim() !== '') add();
        }}
        trailingIcon={
          <IconButton enabled={typed.trim() !== ''} onClick={add} label="Add address">
            <Drawable name="ic_add" />
          </IconButton>
        }
        className="w-full"
      />
    </div>
  );
};

import type { ReactNode } from 'react';
import { type Json, jsonObjects, jsonStrings, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';

export const LINKS_VIEW = 'view.links';

export type LinkType = {
  readonly name: 'Udp' | 'Tcp' | 'Serial' | 'Bluetooth' | 'LogReplay' | 'Mock' | 'AircastCloud' | 'Other';
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
  Other: { name: 'Other', ordinal: 7, id: '' },
} as const satisfies Record<string, LinkType>;

export const LinkTypeEntries: readonly LinkType[] = Object.values(LinkType);

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

export const LinkEditingEntries: readonly LinkEditing[] = Object.values(LinkEditing);

export const linkEditingFrom = (id: string): LinkEditing => LinkEditingEntries.find((it) => it.id === id && it !== LinkEditing.None) ?? LinkEditing.None;

export type SerialFraming = { dataBits: number; stopBits: number; parity: number; flowControl: number };

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

export const linkRows = (view: Json | null): readonly LinkRow[] => {
  const links = optJSONArray(view, 'configured');
  if (links === null) return [];
  return jsonObjects(links).map((link) => ({
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
    framing: { dataBits: optInt(link, 'dataBits', 8), stopBits: optInt(link, 'stopBits', 1), parity: optInt(link, 'parity', 0), flowControl: optInt(link, 'flowControl', 0) },
    servers: jsonStrings(optJSONArray(link, 'hostList')),
    autoConnect: optBoolean(link, 'autoConnect'),
    highLatency: optBoolean(link, 'highLatency'),
    type: linkTypeFrom(optText(link, 'type')),
    filename: optText(link, 'filename'),
  }));
};

export const LinksScreen = (_props: { className?: string; footer?: ReactNode }) => null;

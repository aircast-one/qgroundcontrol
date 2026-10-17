import { useState } from 'react';
import { type Json, isNull, opt, optBoolean, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { delay, format } from '@/kotlin/Std';
import { AlertDialog, DropdownMenu, DropdownMenuItem, OutlinedButton, OutlinedTextField, Text, TextButton } from '@/m3';
import { useKeyedState } from '@/map/SettingStepper';
import { md } from '@/theme/Colors';
import { SectionHeader } from '@/ui/SetupRow';

export const ESP_BRIDGE_VIEW = 'view.espBridge';
export const ESP_BRIDGE_SCREEN = 'espBridge';
const ESP_BRIDGE_OPENED = 'espBridge.open';
const ESP_POLL_MS = 1000;
const WIFI_MODES: readonly string[] = ['Access point mode', 'Station mode'];
const STATION_MODE = 1;
const HOST_PORTS = { first: 1024, last: 65535 } as const;
const WIFI_CHANNELS: readonly string[] = Array.from({ length: 11 }, (_, at) => String(at + 1));

export type LinkCounts = { received: string; lost: string; sent: string };

export const LinkCounts = (received: string, lost: string, sent: string): LinkCounts => ({ received, lost, sent });

export type EspBridge = {
  modeIndex: number | null;
  modePath: string;
  channel: number;
  channelPath: string;
  ssid: string;
  password: string;
  ssidSta: string | null;
  passwordSta: string | null;
  baudRates: readonly number[];
  baudIndex: number;
  hostPort: string;
  hostPortPath: string;
  vehicle: LinkCounts;
  bridge: LinkCounts;
  qgc: LinkCounts;
  rebootPrompt: string;
  busy: boolean;
};

export const grouped = (value: unknown): string => (typeof value === 'number' && Number.isFinite(value) ? format('%,d', Math.trunc(value)) : '');

const counts = (json: Json | null): LinkCounts => LinkCounts(grouped(opt(json, 'received')), grouped(opt(json, 'lost')), grouped(opt(json, 'sent')));

const longAt = (value: unknown): number => {
  const parsed = typeof value === 'number' ? value : typeof value === 'string' && value.trim() !== '' ? Number(value) : NaN;
  return Number.isFinite(parsed) ? Math.trunc(parsed) : 0;
};

export const espBridge = (view: Json | null): EspBridge | null => {
  if (view === null || !optBoolean(view, 'available')) return null;
  const status = optJSONObject(view, 'status');
  const hostPort = optJSONObject(view, 'hostPort');
  return {
    modeIndex: isNull(view, 'modeIndex') ? null : optInt(view, 'modeIndex'),
    modePath: optText(view, 'modePath'),
    channel: optInt(view, 'channel', 1),
    channelPath: optText(view, 'channelPath'),
    ssid: optText(view, 'ssid'),
    password: optText(view, 'password'),
    ssidSta: isNull(view, 'ssidSta') ? null : optText(view, 'ssidSta'),
    passwordSta: isNull(view, 'passwordSta') ? null : optText(view, 'passwordSta'),
    baudRates: (optJSONArray(view, 'baudRates') ?? []).map(longAt),
    baudIndex: optInt(view, 'baudIndex', 4),
    hostPort: hostPort === null ? '' : optText(hostPort, 'valueString'),
    hostPortPath: hostPort === null ? '' : optText(hostPort, 'path'),
    vehicle: counts(optJSONObject(status, 'vehicle')),
    bridge: counts(optJSONObject(status, 'bridge')),
    qgc: counts(optJSONObject(status, 'qgc')),
    rebootPrompt: optText(view, 'rebootPrompt'),
    busy: optBoolean(view, 'busy'),
  };
};

export const stationFieldsEnabled = (bridge: EspBridge): boolean => bridge.modeIndex === STATION_MODE;

export const hostPortTyped = (typed: string): number | null => {
  const port = /^[+-]?\d+$/.test(typed) ? Number(typed) : NaN;
  return port >= HOST_PORTS.first && port <= HOST_PORTS.last ? port : null;
};

export const EspBridgeScreen = ({ className }: { className?: string }) => {
  const [revision, setRevision] = useState(0);
  const [read, setRead] = useState<EspBridge | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);
  const [confirmReboot, setConfirmReboot] = useState(false);
  useLaunchedEffect([], () => void Qgc.invoke(ESP_BRIDGE_OPENED));
  useLaunchedEffect([revision], async (scope) => {
    const next = espBridge(await Qgc.get(ESP_BRIDGE_VIEW));
    if (!scope.isActive()) return;
    setRead(next);
    await delay(ESP_POLL_MS);
    if (scope.isActive()) setRevision((it) => it + 1);
  });
  const act = async (path: string, ...args: unknown[]) => setRefusal(await Qgc.refusalOf(path, ...args));
  const set = async (path: string, value: unknown) => setRefusal(await Qgc.writeRefusal(path, value));
  const bridge = read;
  if (bridge === null) return <Text className={`p-4 ${className ?? ''}`}>No WiFi bridge is connected.</Text>;
  const links: readonly (readonly [string, LinkCounts])[] = [
    ['Bridge/vehicle link', bridge.vehicle],
    ['Bridge/QGC link', bridge.bridge],
    ['QGC/bridge link', bridge.qgc],
  ];
  return (
    <>
      <div className={`flex w-full flex-col gap-2 overflow-y-auto px-5 py-3 ${className ?? ''}`}>
        <SectionHeader text="ESP WiFi bridge settings" />
        {bridge.modeIndex !== null ? <Choice label="WiFi mode" options={WIFI_MODES} index={bridge.modeIndex} onPick={(it) => void set(`${bridge.modePath}.rawValue`, it)} /> : null}
        <Choice label="WiFi channel" options={WIFI_CHANNELS} index={bridge.channel - 1} enabled={(bridge.modeIndex ?? 0) === 0} onPick={(it) => void set(`${bridge.channelPath}.rawValue`, it + 1)} />
        <TextSetting label="WiFi AP SSID" value={bridge.ssid} onDone={(it) => void act('espBridge.setText', 'ssid', it)} />
        <TextSetting label="WiFi AP password" value={bridge.password} onDone={(it) => void act('espBridge.setText', 'password', it)} />
        <TextSetting label="WiFi STA SSID" value={bridge.ssidSta ?? ''} enabled={stationFieldsEnabled(bridge)} onDone={(it) => void act('espBridge.setText', 'ssidSta', it)} />
        <TextSetting label="WiFi STA password" value={bridge.passwordSta ?? ''} enabled={stationFieldsEnabled(bridge)} onDone={(it) => void act('espBridge.setText', 'passwordSta', it)} />
        <Choice label="UART baud rate" options={bridge.baudRates.map(String)} index={bridge.baudIndex} onPick={(it) => void act('espBridge.baud', it)} />
        <TextSetting
          label="QGC UDP port"
          value={bridge.hostPort}
          enabled={bridge.hostPortPath !== ''}
          digits
          onDone={(typed) => {
            const port = hostPortTyped(typed);
            if (port !== null) void set(bridge.hostPortPath, port);
          }}
        />

        <SectionHeader text="ESP WiFi bridge status" />
        {links.map(([title, link]) => (
          <div key={title} className="flex flex-col gap-2">
            <Text typography="titleSmall">{title}</Text>
            <StatusRow label="Messages received" value={link.received} />
            <StatusRow label="Messages lost" value={link.lost} />
            <StatusRow label="Messages sent" value={link.sent} />
          </div>
        ))}
        {refusal !== null ? <Text color={md('error')}>{refusal}</Text> : null}
        <div className="flex flex-wrap gap-2">
          <OutlinedButton onClick={() => void act('espBridge.restoreDefaults')}>Restore defaults</OutlinedButton>
          <OutlinedButton enabled={!bridge.busy} onClick={() => setConfirmReboot(true)}>
            Restart WiFi bridge
          </OutlinedButton>
          <OutlinedButton onClick={() => void act('espBridge.resetCounters')}>Reset counters</OutlinedButton>
        </div>
      </div>
      {confirmReboot ? (
        <AlertDialog
          onDismissRequest={() => setConfirmReboot(false)}
          title="Reboot WiFi bridge"
          text={bridge.rebootPrompt}
          confirmButton={
            <TextButton
              onClick={() => {
                setConfirmReboot(false);
                void act('espBridge.reboot');
              }}
            >
              Yes
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setConfirmReboot(false)}>No</TextButton>}
        />
      ) : null}
    </>
  );
};

const StatusRow = ({ label, value }: { label: string; value: string }) => (
  <div className="flex w-full">
    <Text typography="bodySmall" className="flex-1">
      {label}
    </Text>
    <Text typography="bodySmall">{value}</Text>
  </div>
);

const Choice = ({ label, options, index, enabled = true, onPick }: { label: string; options: readonly string[]; index: number; enabled?: boolean; onPick: (index: number) => void }) => {
  const [open, setOpen] = useState(false);
  return (
    <div className="flex w-full items-center">
      <Text className="flex-1">{label}</Text>
      <div className="relative">
        <OutlinedButton enabled={enabled} onClick={() => setOpen(true)}>
          {options[index] ?? ''}
        </OutlinedButton>
        <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
          {options.map((option, at) => (
            <DropdownMenuItem
              key={at}
              text={option}
              onClick={() => {
                setOpen(false);
                onPick(at);
              }}
            />
          ))}
        </DropdownMenu>
      </div>
    </div>
  );
};

const TextSetting = ({ label, value, enabled = true, digits = false, onDone }: { label: string; value: string; enabled?: boolean; digits?: boolean; onDone: (typed: string) => void }) => {
  const [typed, setTyped] = useKeyedState([value], () => value);
  return (
    <div className="flex w-full items-center">
      <Text className="flex-1">{label}</Text>
      <OutlinedTextField
        value={typed}
        onValueChange={(next) => setTyped(digits ? next.replace(/\D/g, '').slice(0, 5) : next.slice(0, 16))}
        enabled={enabled}
        singleLine
        keyboardType={digits ? 'number' : 'text'}
        onDone={() => onDone(typed)}
        className="w-[180px]"
      />
    </div>
  );
};

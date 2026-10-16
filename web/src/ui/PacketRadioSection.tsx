import { useState } from 'react';
import { SetupCommands } from '@/bridge/BridgeCommands';
import { type Json, type JsonArray, isNull, jsonStrings, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { DropdownMenu, DropdownMenuItem, OutlinedButton, Text } from '@/m3';

export const PACKET_RADIO_VIEW = 'view.packetRadio';
const PACKET_RADIO_POLL_MS = 1000;
const PACKET_RADIO_DEVICE = 'settings.packetRadioSettings.deviceName.rawValue';
const AUTOMATIC_ADAPTER = 'Automatic';
const WAITING = 'waiting';

export type PacketRadioStatus = {
  statusText: string;
  linkActive: boolean;
  haveSignal: boolean;
  signal: string;
  noise: string;
  linkScore: string;
  packetLoss: string;
  videoPackets: string;
  adapters: readonly string[];
};

const joined = (values: JsonArray | null): string => (values ?? []).map((value) => String(value)).join('  |  ');

export const packetRadioStatus = (view: Json | null): PacketRadioStatus | null => {
  if (view === null || optText(view, 'class') !== 'PacketRadio') return null;
  const signal = optBoolean(view, 'haveSignal');
  const statusText = optText(view, 'statusText');
  return {
    statusText: statusText.trim() === '' ? 'Unavailable' : statusText,
    linkActive: optBoolean(view, 'linkActive'),
    haveSignal: signal,
    signal: signal ? joined(optJSONArray(view, 'antennaRssiRaw')) : WAITING,
    noise: signal ? `${joined(optJSONArray(view, 'antennaSnr'))} dB` : WAITING,
    linkScore: signal && !isNull(view, 'linkScore') ? optInt(view, 'linkScore').toString() : WAITING,
    packetLoss: isNull(view, 'packetLoss') ? '' : optInt(view, 'packetLoss').toString(),
    videoPackets: isNull(view, 'videoPackets') ? '' : optInt(view, 'videoPackets').toString(),
    adapters: jsonStrings(optJSONArray(view, 'adapters')),
  };
};

export const PacketRadioSection = ({ onWrite }: { onWrite: () => void }) => {
  const [read, setRead] = useState<PacketRadioStatus | null>(null);
  const [device, setDevice] = useState('');
  const [choosing, setChoosing] = useState(false);
  useLaunchedEffect([], async (scope) => {
    await SetupCommands.refreshPacketRadios();
    const poll = async (): Promise<void> => {
      if (!scope.isActive()) return;
      const status = packetRadioStatus(await Qgc.get(PACKET_RADIO_VIEW));
      if (scope.isActive()) setRead(status);
      const named = optText(await Qgc.get(PACKET_RADIO_DEVICE), 'value');
      if (scope.isActive()) setDevice(named);
      await delay(PACKET_RADIO_POLL_MS);
      return poll();
    };
    await poll();
  });
  const status = read;
  return (
    <div className="flex w-full flex-col gap-1.5 px-5 py-2">
      <StatusRow label="Status" value={status?.statusText ?? 'Unavailable'} />
      {status?.linkActive === true ? (
        <>
          <Text typography="titleSmall">Link quality</Text>
          <StatusRow label="Signal per antenna (0-126)" value={status.signal} />
          <StatusRow label="Noise margin per antenna" value={status.noise} />
          <StatusRow label="Link score (1000-2000)" value={status.linkScore} />
          <StatusRow label="Packets lost (last second)" value={status.packetLoss} />
          <StatusRow label="Video packets" value={status.videoPackets} />
        </>
      ) : null}
      <Text typography="titleSmall">Radio</Text>
      <div className="flex w-full items-center">
        <Text className="flex-1">Wi-Fi adapter</Text>
        <div className="relative">
          <OutlinedButton onClick={() => setChoosing(true)}>{device.trim() === '' ? AUTOMATIC_ADAPTER : device}</OutlinedButton>
          <DropdownMenu expanded={choosing} onDismissRequest={() => setChoosing(false)}>
            {[AUTOMATIC_ADAPTER, ...(status?.adapters ?? [])].map((name, index) => (
              <DropdownMenuItem
                key={`${name}-${index}`}
                text={name}
                onClick={() => {
                  setChoosing(false);
                  void (async () => {
                    await Qgc.set(PACKET_RADIO_DEVICE, index === 0 ? '' : name);
                    onWrite();
                  })();
                }}
              />
            ))}
          </DropdownMenu>
        </div>
      </div>
    </div>
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

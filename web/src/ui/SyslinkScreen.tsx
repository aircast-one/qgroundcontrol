import { useState } from 'react';
import { type Json, jsonStrings, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useKeyedState, useLaunchedEffect } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { type KeyboardType, OutlinedButton, OutlinedTextField, Text } from '@/m3';
import { md } from '@/theme/Colors';
import { ChoiceField } from '@/ui/SettingsScreen';
import { SectionHeader } from '@/ui/SetupRow';

export const SYSLINK_SCREEN = 'syslink';
const SYSLINK_VIEW = 'view.syslink';
const SYSLINK_POLL_MS = 1000;
const HEX = /^[0-9A-Fa-f]{0,10}$/;
const INT_MAX = 2147483647;

export type Syslink = {
  channel: number;
  channelHint: string;
  address: string;
  addressHint: string;
  rate: number;
  rates: readonly string[];
};

export const syslink = (view: Json | null): Syslink | null =>
  view !== null && optBoolean(view, 'available')
    ? {
        channel: optInt(view, 'channel'),
        channelHint: optText(view, 'channelHint'),
        address: optText(view, 'address'),
        addressHint: optText(view, 'addressHint'),
        rate: optInt(view, 'rate', -1),
        rates: jsonStrings(optJSONArray(view, 'rates')),
      }
    : null;

export const hexAddress = (typed: string): boolean => HEX.test(typed);

const digitsOnly = (typed: string): boolean => /^\d*$/.test(typed);

const intOrNull = (typed: string): number | null => (/^\d+$/.test(typed) && Number(typed) <= INT_MAX ? Number(typed) : null);

export const SyslinkScreen = ({ className }: { className?: string }) => {
  const [revision, setRevision] = useState(0);
  const [read, setRead] = useState<Syslink | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);
  useLaunchedEffect([revision], async (scope) => {
    const next = syslink(await Qgc.get(SYSLINK_VIEW));
    if (!scope.isActive()) return;
    setRead(next);
    await delay(SYSLINK_POLL_MS);
    if (scope.isActive()) setRevision((it) => it + 1);
  });
  const act = async (path: string, ...args: unknown[]) => {
    setRefusal(await Qgc.refusalOf(path, ...args));
    setRead(syslink(await Qgc.get(SYSLINK_VIEW)));
  };
  const radio = read;
  if (radio === null) return <Text className={`p-4 ${className ?? ''}`}>This vehicle has no Syslink radio.</Text>;
  return (
    <div className={`flex w-full flex-col gap-2 overflow-y-auto px-5 py-3 ${className ?? ''}`}>
      <SectionHeader text="Radio Settings" />
      <RadioField
        label="Channel"
        value={String(radio.channel)}
        hint={radio.channelHint}
        keyboard="number"
        accept={digitsOnly}
        onDone={(typed) => {
          const channel = intOrNull(typed);
          if (channel !== null) void act('syslink.setChannel', channel);
        }}
      />
      <RadioField label="Address" value={radio.address} hint={radio.addressHint} keyboard="text" accept={hexAddress} onDone={(typed) => void act('syslink.setAddress', typed)} />
      <ChoiceField label="Data Rate" value={radio.rates[radio.rate] ?? ''} options={radio.rates} onPick={(it) => void act('syslink.setRate', it)} />
      {refusal !== null ? <Text color={md('error')}>{refusal}</Text> : null}
      <OutlinedButton className="self-start" onClick={() => void act('syslink.resetDefaults')}>
        Restore defaults
      </OutlinedButton>
    </div>
  );
};

const RadioField = ({ label, value, hint, keyboard, accept, onDone }: { label: string; value: string; hint: string; keyboard: KeyboardType; accept: (typed: string) => boolean; onDone: (typed: string) => void }) => {
  const [typed, setTyped] = useKeyedState([value], () => value);
  return (
    <div className="flex flex-col">
      <div className="flex w-full items-center">
        <Text className="flex-1">{label}</Text>
        <div
          className="w-[180px]"
          onBlur={(event) => {
            if (!event.currentTarget.contains(event.relatedTarget) && typed !== value) onDone(typed);
          }}
        >
          <OutlinedTextField value={typed} onValueChange={(it) => accept(it) && setTyped(it)} singleLine keyboardType={keyboard} onDone={() => onDone(typed)} />
        </div>
      </div>
      <Text typography="bodySmall" color={md('onSurfaceVariant')}>
        {hint}
      </Text>
    </div>
  );
};

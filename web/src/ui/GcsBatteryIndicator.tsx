import { useEffect, useState } from 'react';
import { type Json, has, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { ListItem, Text } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { Drawable } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';

const BATTERY_STATUS_CHARGING = 2;
const BATTERY_STATUS_DISCHARGING = 3;
const BATTERY_STATUS_FULL = 5;

export type PhoneBattery = { percent: number; charging: boolean };

export const phoneBattery = (level: number, scale: number, status: number): PhoneBattery | null =>
  level < 0 || scale <= 0 ? null : { percent: Math.trunc((level * 100) / scale), charging: status === BATTERY_STATUS_CHARGING || status === BATTERY_STATUS_FULL };

export const gcsBatteryPath = (battery: PhoneBattery): string => `view.gcsBattery(${battery.percent},${battery.charging})`;

export type GcsBatteryReading = { state: string; levelText: string; stateText: string; heading: string; title: string };

export const gcsBatteryReading = (view: Json | null): GcsBatteryReading | null =>
  view !== null && has(view, 'state')
    ? { state: optText(view, 'state'), levelText: optText(view, 'levelText'), stateText: optText(view, 'stateText'), heading: optText(view, 'heading'), title: optText(view, 'title') }
    : null;

const batteryColour = (state: string): string => {
  switch (state) {
    case 'charging':
      return aircast('success');
    case 'critical':
      return md('error');
    case 'low':
      return aircast('warning');
    default:
      return md('onSurfaceVariant');
  }
};

type BatteryManager = EventTarget & { readonly level: number; readonly charging: boolean };

const getBattery = (): Promise<BatteryManager> | null => {
  const host = navigator as Navigator & { getBattery?: () => Promise<BatteryManager> };
  return host.getBattery?.() ?? null;
};

export const useRememberGcsBattery = (): GcsBatteryReading | null => {
  const [battery, setBattery] = useState<PhoneBattery | null>(null);
  const [reading, setReading] = useState<GcsBatteryReading | null>(null);

  useEffect(() => {
    const live = { manager: null as BatteryManager | null, disposed: false };
    const receive = () => {
      const manager = live.manager;
      if (manager !== null) setBattery(phoneBattery(Math.round(manager.level * 100), 100, manager.charging ? BATTERY_STATUS_CHARGING : BATTERY_STATUS_DISCHARGING));
    };
    void getBattery()
      ?.then((manager) => {
        if (live.disposed) return;
        live.manager = manager;
        manager.addEventListener('levelchange', receive);
        manager.addEventListener('chargingchange', receive);
        receive();
      })
      .catch(() => undefined);
    return () => {
      live.disposed = true;
      live.manager?.removeEventListener('levelchange', receive);
      live.manager?.removeEventListener('chargingchange', receive);
    };
  }, []);

  useLaunchedEffect([battery?.percent, battery?.charging], async (scope) => {
    if (battery === null) return;
    const next = gcsBatteryReading(await Qgc.get(gcsBatteryPath(battery)));
    if (scope.isActive()) setReading(next);
  });
  return reading;
};

export const GcsBatteryCell = ({ reading }: { reading: GcsBatteryReading | null }) => {
  const [open, setOpen] = useState(false);
  if (reading === null) return null;
  const tint = batteryColour(reading.state);
  return (
    <>
      <button type="button" className="flex items-center gap-0.5" aria-label={`${reading.title} ${reading.levelText}`} title={reading.heading} onClick={() => setOpen(true)}>
        <Drawable name="ic_smartphone" size={14} style={{ color: tint }} />
        <Text typography="labelMedium" color={tint} maxLines={1}>
          {reading.levelText}
        </Text>
      </button>

      {open ? (
        <AircastSheet onDismissRequest={() => setOpen(false)}>
          <div className="flex w-full flex-col pb-6">
            <Text typography="titleMedium" className="px-sheet py-2">
              {reading.heading}
            </Text>
            <ListItem headlineContent="Charge" trailingContent={reading.levelText} />
            <ListItem headlineContent="State" trailingContent={reading.stateText} />
          </div>
        </AircastSheet>
      ) : null}
    </>
  );
};

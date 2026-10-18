import type { ReactNode } from 'react';
import { TelemetryScreen } from '@/ui/DeviceTelemetryScreen';

export type DeviceScreenProps = { readonly href: string; readonly query: URLSearchParams; readonly onOpen: (href: string) => void; readonly onClose: () => void };

export type DeviceScreenEntry = { readonly title: string; readonly screen: (props: DeviceScreenProps) => ReactNode };

export const DEVICE_SCREENS: ReadonlyMap<string, DeviceScreenEntry> = new Map<string, DeviceScreenEntry>([['/telemetry', { title: 'Telemetry', screen: TelemetryScreen }]]);

export const devicePath = (href: string): { path: string; query: URLSearchParams } => {
  const url = new URL(href, 'http://device');
  return { path: url.pathname, query: url.searchParams };
};

export const deviceLinkOf = (hash: string): string | null => {
  const href = new URLSearchParams(hash.replace(/^#/, '')).get('device');
  return href !== null && href.startsWith('/') ? href : null;
};

import type { ReactNode } from 'react';
import { ConnectScreen } from '@/ui/DeviceConnectScreen';
import { FlightsScreen } from '@/ui/DeviceFlightsScreen';
import { EngineeringScreen } from '@/ui/DeviceEngineeringScreen';
import { RecordingsScreen } from '@/ui/DeviceRecordingsScreen';
import { DeviceSettingsScreen } from '@/ui/DeviceSettingsScreen';
import { NetworkScreen } from '@/ui/DeviceNetworkScreen';
import { CameraSetupScreen } from '@/ui/DeviceCameraSetupScreen';
import { CamerasScreen } from '@/ui/DeviceCamerasScreen';
import { SetupScreen } from '@/ui/DeviceSetupScreen';
import { TelemetryScreen } from '@/ui/DeviceTelemetryScreen';
import { VideoScreen } from '@/ui/DeviceVideoScreen';
import { DetectionScreen } from '@/ui/DeviceDetectionScreen';

export type DeviceScreenProps = {
  readonly href: string;
  readonly query: URLSearchParams;
  readonly onOpen: (href: string) => void;
  readonly onOpenFrom: (href: string, returnTo: string) => void;
  readonly onClose: () => void;
};

export type DeviceScreenEntry = { readonly title: string; readonly screen: (props: DeviceScreenProps) => ReactNode; readonly narrow?: boolean };

export const DEVICE_SCREENS: ReadonlyMap<string, DeviceScreenEntry> = new Map<string, DeviceScreenEntry>([
  ['/telemetry', { title: 'Telemetry', screen: TelemetryScreen }],
  ['/connect', { title: 'Ground station links', screen: ConnectScreen }],
  ['/flights', { title: 'Flight log', screen: FlightsScreen }],
  ['/settings/device', { title: 'Device', screen: DeviceSettingsScreen }],
  ['/settings/engineering', { title: 'Engineering', screen: EngineeringScreen }],
  ['/settings/video', { title: 'Video engine', screen: VideoScreen }],
  ['/settings/detection', { title: 'Detection', screen: DetectionScreen }],
  ['/settings/recordings', { title: 'Recordings', screen: RecordingsScreen }],
  ['/network', { title: 'Connection', screen: NetworkScreen }],
  ['/streaming', { title: 'Cameras', screen: CamerasScreen }],
  ['/settings/camera', { title: 'Sharing & recording', screen: CameraSetupScreen }],
  ['/setup', { title: 'Device setup', screen: SetupScreen, narrow: true }],
]);

export const devicePath = (href: string): { path: string; query: URLSearchParams } => {
  const url = new URL(href, 'http://device');
  return { path: url.pathname, query: url.searchParams };
};

export const deviceLinkOf = (hash: string): string | null => {
  const href = new URLSearchParams(hash.replace(/^#/, '')).get('device');
  return href !== null && href.startsWith('/') ? href : null;
};

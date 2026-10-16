import type { DrawableName } from '@/res/drawables';

export type AnalyzeSection = { readonly name: 'FlightData' | 'Live' | 'Vehicle'; readonly ordinal: number; readonly title: string };

export const AnalyzeSection = {
  FlightData: { name: 'FlightData', ordinal: 0, title: 'Flight data' },
  Live: { name: 'Live', ordinal: 1, title: 'Live' },
  Vehicle: { name: 'Vehicle', ordinal: 2, title: 'Vehicle' },
} as const satisfies Record<string, AnalyzeSection>;

export const AnalyzeSectionEntries: readonly AnalyzeSection[] = [AnalyzeSection.FlightData, AnalyzeSection.Live, AnalyzeSection.Vehicle];

export type AnalyzePage = {
  readonly name: 'LogDownload' | 'GeoTag' | 'Vibration' | 'Inspector' | 'Console' | 'Messages' | 'Firmware';
  readonly ordinal: number;
  readonly label: string;
  readonly description: string;
  readonly section: AnalyzeSection;
  readonly icon: DrawableName;
  readonly requiresVehicle: boolean;
};

export const AnalyzePage = {
  LogDownload: { name: 'LogDownload', ordinal: 0, label: 'Flight logs', description: 'Download flight logs from the vehicle', section: AnalyzeSection.FlightData, icon: 'ic_download', requiresVehicle: true },
  GeoTag: { name: 'GeoTag', ordinal: 1, label: 'Geotag images', description: 'Match photos to the flight log', section: AnalyzeSection.FlightData, icon: 'ic_photo_camera', requiresVehicle: false },
  Vibration: { name: 'Vibration', ordinal: 2, label: 'Vibration', description: 'Accelerometer vibration levels and clipping', section: AnalyzeSection.Live, icon: 'ic_vibration', requiresVehicle: true },
  Inspector: { name: 'Inspector', ordinal: 3, label: 'MAVLink inspector', description: 'Live message rates and field values', section: AnalyzeSection.Live, icon: 'ic_analytics', requiresVehicle: true },
  Console: { name: 'Console', ordinal: 4, label: 'Console', description: 'Vehicle shell over MAVLink', section: AnalyzeSection.Live, icon: 'ic_terminal', requiresVehicle: true },
  Messages: { name: 'Messages', ordinal: 5, label: 'Messages', description: 'What the vehicle has said since it connected', section: AnalyzeSection.Live, icon: 'ic_description', requiresVehicle: false },
  Firmware: { name: 'Firmware', ordinal: 6, label: 'Firmware', description: 'Flash a board through its bootloader over USB', section: AnalyzeSection.Vehicle, icon: 'ic_developer_board', requiresVehicle: false },
} as const satisfies Record<string, AnalyzePage>;

export const AnalyzePageEntries: readonly AnalyzePage[] = [
  AnalyzePage.LogDownload,
  AnalyzePage.GeoTag,
  AnalyzePage.Vibration,
  AnalyzePage.Inspector,
  AnalyzePage.Console,
  AnalyzePage.Messages,
  AnalyzePage.Firmware,
];

export const AnalyzeScreen = (_props: { page: AnalyzePage | null; onSelect: (page: AnalyzePage | null) => void; className?: string }) => null;

export type GcsBatteryReading = { state: string; levelText: string; stateText: string; heading: string; title: string };

export const useRememberGcsBattery = (): GcsBatteryReading | null => null;

export const GcsBatteryCell = (_props: { reading: GcsBatteryReading | null }) => null;

export type RtkStatus = {
  active: boolean;
  valid: boolean;
  satellites: number | null;
  durationS: number | null;
  accuracyM: number | null;
  accuracyText: string | null;
  latitude: number | null;
  longitude: number | null;
  altitudeM: number | null;
};

export const useRememberRtkStatus = (): RtkStatus | null => null;

export const RtkIndicatorCell = (_props: { status: RtkStatus | null }) => null;

export const RtkSettingsSheetContent = (_props: { status: RtkStatus | null }) => null;

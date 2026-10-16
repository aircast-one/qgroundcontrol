const UNAVAILABLE = -1;

export const WfbUsb = {
  initialize: (): void => undefined,

  devices: (): readonly string[] => [],

  open: (_deviceName: string): number => UNAVAILABLE,

  close: (_deviceName: string): void => undefined,
};

export const BluetoothLinks = {
  initialize: (): void => undefined,

  devicesInfo: (): readonly string[] => [],

  scan: (_start: number): boolean => false,

  scanning: (): boolean => false,

  open: (_address: string, _id: number): boolean => false,

  write: (_id: number, _data: Uint8Array): boolean => false,

  close: (_id: number): void => undefined,
};

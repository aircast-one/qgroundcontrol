const UNAVAILABLE = -1;
const USB_CLASS_MASS_STORAGE = 0x08;
const SUBCLASS_SCSI = 0x06;
const PROTOCOL_BULK_ONLY = 0x50;

export type UsbInterfaceClass = readonly [number, number, number];

export const UsbDisks = {
  initialize: (): void => undefined,

  isCardReader: (classes: readonly UsbInterfaceClass[]): boolean =>
    classes.some(([cls, subclass, protocol]) => cls === USB_CLASS_MASS_STORAGE && subclass === SUBCLASS_SCSI && protocol === PROTOCOL_BULK_ONLY),

  label: (manufacturer: string | null, product: string | null): string => {
    const named = [...new Set([manufacturer, product].filter((it): it is string => it !== null).map((it) => it.trim()).filter((it) => it !== ''))].join(' ');
    return (named.trim() === '' ? 'USB card reader' : named).replaceAll('\t', ' ');
  },

  devices: (): readonly string[] => [],

  open: (_deviceName: string): number => UNAVAILABLE,

  close: (_deviceName: string): void => undefined,
};

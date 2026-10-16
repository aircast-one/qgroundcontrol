import { describe, expect, it } from 'vitest';
import { UsbDisks } from './UsbDisks';

describe('UsbDisksTest', () => {
  it('only_bulk_only_scsi_mass_storage_counts_as_a_card_reader', () => {
    expect(UsbDisks.isCardReader([
      [0x03, 1, 1],
      [0x08, 0x06, 0x50],
    ])).toBe(true);
    expect(UsbDisks.isCardReader([[0x08, 0x06, 0x62]])).toBe(false);
    expect(UsbDisks.isCardReader([[0xff, 0, 0]])).toBe(false);
    expect(UsbDisks.isCardReader([])).toBe(false);
  });

  it('the_label_names_the_reader_without_repeating_or_breaking_the_list_format', () => {
    expect(UsbDisks.label('Generic', 'SD Reader')).toBe('Generic SD Reader');
    expect(UsbDisks.label('Kingston', 'Kingston')).toBe('Kingston');
    expect(UsbDisks.label(null, '  ')).toBe('USB card reader');
    expect(UsbDisks.label('A\tB', null)).toBe('A B');
  });
});

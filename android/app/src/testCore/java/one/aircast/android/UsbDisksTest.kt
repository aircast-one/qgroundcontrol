package one.aircast.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class UsbDisksTest {
    @Test
    fun only_bulk_only_scsi_mass_storage_counts_as_a_card_reader() {
        assertTrue(UsbDisks.isCardReader(listOf(Triple(0x03, 1, 1), Triple(0x08, 0x06, 0x50))))
        assertFalse(UsbDisks.isCardReader(listOf(Triple(0x08, 0x06, 0x62))))
        assertFalse(UsbDisks.isCardReader(listOf(Triple(0xff, 0, 0))))
        assertFalse(UsbDisks.isCardReader(emptyList()))
    }

    @Test
    fun the_label_names_the_reader_without_repeating_or_breaking_the_list_format() {
        assertEquals("Generic SD Reader", UsbDisks.label("Generic", "SD Reader"))
        assertEquals("Kingston", UsbDisks.label("Kingston", "Kingston"))
        assertEquals("USB card reader", UsbDisks.label(null, "  "))
        assertEquals("A B", UsbDisks.label("A\tB", null))
    }
}

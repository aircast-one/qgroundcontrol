package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ParameterLinksTest {
    @Test
    fun `a param link names the parameter and other links are left to the browser`() {
        assertEquals("COM_ARM_WO_GPS", paramLinkName("param://COM_ARM_WO_GPS"))
        assertNull(paramLinkName("param://"))
        assertNull(paramLinkName("https://docs.px4.io"))
    }

    @Test
    fun `a read-only parameter says so until force edit is on`() {
        assertNull(forceEditNote(readOnly = false, forced = false))
        assertEquals(READ_ONLY_NOTE, forceEditNote(readOnly = true, forced = false))
        assertEquals(FORCE_EDIT_NOTE, forceEditNote(readOnly = true, forced = true))
    }
}

package one.aircast.android.ui

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SupportHostTest {

    @Test
    fun `the shipped default is refused rather than dialled`() {
        assertFalse(
            "Mavlink.SettingsGroup.json ships 'support.ardupilot.org:xxxx' as the VALUE of this " +
                "setting, not as a hint. It is non-blank and has no spaces, so the old check " +
                "passed it, the dialog offered to forward telemetry there, and the link was made " +
                "to port 0 - a write that succeeds and goes nowhere",
            supportHostIsUsable("support.ardupilot.org:xxxx"),
        )
    }

    @Test
    fun `a real host and port is accepted`() {
        assertTrue(supportHostIsUsable("support.ardupilot.org:14550"))
        assertTrue(supportHostIsUsable("127.0.0.1:5760"))
    }

    @Test
    fun `a host with no port at all is still accepted`() {
        assertTrue(
            "the field has always allowed a bare host; refusing it would be a new rule rather " +
                "than a fix, and the port the link picks is not this screen's decision",
            supportHostIsUsable("support.ardupilot.org"),
        )
    }

    @Test
    fun `ports outside the range are refused the way the core refuses them`() {
        assertFalse("link_form_view says 1 to 65535", supportHostIsUsable("host:0"))
        assertFalse(supportHostIsUsable("host:65536"))
        assertFalse(supportHostIsUsable("host:-1"))
    }

    @Test
    fun `blank and spaced hosts are refused as before`() {
        assertFalse(supportHostIsUsable(""))
        assertFalse(supportHostIsUsable("   "))
        assertFalse(supportHostIsUsable("two words:14550"))
    }
}

package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class LinksScreenTest {
    private fun payload(vararg elements: String) =
        JSONObject("""{"elements":[${elements.joinToString(",")}]}""")

    private val autoConnectUdp =
        """{"name":"UDP Link (AutoConnect)","summary":"UDP port 14550","dynamic":true,"children":["link"]}"""
    private val savedTcp =
        """{"name":"Pi","summary":"TCP 10.0.0.4:5760","dynamic":false,"children":["link"],"heardVehicle":true}"""
    private val savedIdle =
        """{"name":"Bench","summary":"UDP port 14551","dynamic":false,"children":[]}"""

    @Test
    fun `automatic links are not offered for the user to manage`() {
        val rows = configuredRows(linkRows(payload(autoConnectUdp, savedTcp)))
        assertEquals(listOf("Pi"), rows.map { it.name })
    }

    @Test
    fun `a row keeps the index of its position in the full list`() {
        val rows = configuredRows(linkRows(payload(autoConnectUdp, savedTcp)))
        assertEquals(1, rows.single().index)
    }

    @Test
    fun `connection state is read from the link child`() {
        val rows = linkRows(payload(savedTcp, savedIdle))
        assertEquals(listOf(true, false), rows.map { it.connected })
    }

    @Test
    fun `a last error is carried through`() {
        val rows = linkRows(payload("""{"name":"X","dynamic":false,"lastError":"Connection refused"}"""))
        assertEquals("Connection refused", rows.single().lastError)
    }

    @Test
    fun `an absent payload yields no rows`() {
        assertEquals(emptyList<LinkRow>(), linkRows(null))
        assertEquals(emptyList<LinkRow>(), linkRows(JSONObject("{}")))
    }

    @Test
    fun `a link names itself from what it points at`() {
        assertEquals("UDP 14550", autoLinkName("udp", "", "14550"))
        assertEquals("TCP 10.0.0.4:5760", autoLinkName("tcp", "10.0.0.4", "5760"))
    }

    @Test
    fun `a port outside the valid range is refused`() {
        assertEquals("Port must be a number between 1 and 65535.", linkFormError("udp", "", "0"))
        assertEquals("Port must be a number between 1 and 65535.", linkFormError("udp", "", "70000"))
        assertEquals("Port must be a number between 1 and 65535.", linkFormError("udp", "", "abc"))
    }

    @Test
    fun `tcp without an address is refused and udp without one is fine`() {
        assertEquals(
            "A TCP link needs the address of the device to call.",
            linkFormError("tcp", "", "5760"),
        )
        assertNull(linkFormError("udp", "", "14550"))
    }

    @Test
    fun `a status line does not repeat what the name already says`() {
        val row = LinkRow(0, "TCP 10.0.0.4:5760", "10.0.0.4:5760", false, false, "", false)
        assertEquals("Not connected", linkStatusLine(row))
    }

    @Test
    fun `a status line adds detail a custom name leaves out`() {
        val row = LinkRow(0, "Pi", "TCP 10.0.0.4:5760", true, false, "", true)
        assertEquals("Receiving from the vehicle · TCP 10.0.0.4:5760", linkStatusLine(row))
    }

    @Test
    fun `a status line stands alone when there is no summary`() {
        assertEquals("Receiving from the vehicle", linkStatusLine(LinkRow(0, "Pi", "", true, false, "", true)))
    }

    @Test
    fun `an open link that has heard nothing does not claim to be connected`() {
        val row = LinkRow(0, "Bench", "UDP port 14999", true, false, "", false)
        assertEquals("Open · nothing received yet · UDP port 14999", linkStatusLine(row))
    }

    @Test
    fun `heard is read from the field the core publishes`() {
        assertEquals(listOf(true, false), linkRows(payload(savedTcp, savedIdle)).map { it.heard })
    }

    @Test
    fun `a payload using an invented name for heard yields nothing heard`() {
        val invented =
            """{"name":"X","dynamic":false,"children":["link"],"heard":true,"receiving":true}"""
        assertEquals(false, linkRows(payload(invented)).single().heard)
    }
}

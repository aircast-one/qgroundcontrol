package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class LinksScreenTest {
    private fun view(vararg configured: String) =
        JSONObject("""{"configured":[${configured.joinToString(",")}]}""")

    private val heardTcp =
        """{"index":1,"name":"Pi","statusLine":"Connected · TCP 10.0.0.4:5760",
            "connected":true,"heardVehicle":true,"lastError":""}"""
    private val waitingUdp =
        """{"index":2,"name":"Bench","statusLine":"Waiting for the vehicle · UDP port 14551",
            "connected":true,"heardVehicle":false,"lastError":""}"""
    private val idle =
        """{"index":3,"name":"Old","statusLine":"Not connected","connected":false,
            "heardVehicle":false,"lastError":"Connection refused"}"""

    @Test
    fun `the sentence comes from the core, not from the head`() {
        val rows = linkRows(view(heardTcp, waitingUdp, idle))
        assertEquals(
            listOf(
                "Connected · TCP 10.0.0.4:5760",
                "Waiting for the vehicle · UDP port 14551",
                "Not connected",
            ),
            rows.map { it.statusLine },
        )
    }

    @Test
    fun `a row keeps the index the core gave it, not its position in the list`() {
        assertEquals(listOf(1, 2, 3), linkRows(view(heardTcp, waitingUdp, idle)).map { it.index })
    }

    @Test
    fun `heard is what decides emphasis, and it is not the same as connected`() {
        val rows = linkRows(view(heardTcp, waitingUdp))
        assertEquals(listOf(true, true), rows.map { it.connected })
        assertEquals(listOf(true, false), rows.map { it.heard })
    }

    @Test
    fun `the core has already dropped the automatic links`() {
        val withDynamic = JSONObject(
            """{"links":[{"index":0,"name":"UDP Link (AutoConnect)","dynamic":true}],
                "configured":[$heardTcp]}""",
        )
        assertEquals(listOf("Pi"), linkRows(withDynamic).map { it.name })
    }

    @Test
    fun `a last error is carried through`() {
        assertEquals("Connection refused", linkRows(view(idle)).single().lastError)
    }

    @Test
    fun `an absent view yields no rows`() {
        assertEquals(emptyList<LinkRow>(), linkRows(null))
        assertEquals(emptyList<LinkRow>(), linkRows(JSONObject("{}")))
    }

    @Test
    fun `reading the unfiltered list instead of the filtered one yields nothing`() {
        val onlyLinks = JSONObject("""{"links":[$heardTcp]}""")
        assertTrue(linkRows(onlyLinks).isEmpty())
    }

    @Test
    fun `a payload using invented names for the sentence leaves it blank`() {
        val invented =
            """{"index":0,"name":"X","status":"Connected","state":"Connected","connected":true}"""
        assertEquals("", linkRows(view(invented)).single().statusLine)
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
}

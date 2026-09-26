package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
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

class SerialLinkFormTest {
    private fun ports(vararg pairs: Pair<String, String>) = org.json.JSONObject(
        """{"kind":"object","serialPorts":[""" +
            pairs.joinToString(",") { (port, label) -> """{"port":"$port","label":"$label"}""" } + "]}",
    )

    @Test
    fun `each port keeps the label the core paired it with`() {
        val choices = serialPortChoices(ports("/dev/ttyUSB0" to "FTDI UART", "/dev/ttyACM0" to "/dev/ttyACM0"))
        assertEquals(listOf("FTDI UART", "/dev/ttyACM0"), choices.map { it.label })
        assertEquals("/dev/ttyUSB0", choices[0].port)
    }

    @Test
    fun `a blank label or port does not blank the row`() {
        val choices = serialPortChoices(ports("/dev/ttyUSB0" to "", "" to "ghost"))
        assertEquals(listOf(SerialPortChoice("/dev/ttyUSB0", "/dev/ttyUSB0")), choices)
        assertEquals(emptyList<SerialPortChoice>(), serialPortChoices(null))
    }

    @Test
    fun `no port picked is refused`() {
        assertEquals(
            "Pick the port the radio is plugged into.",
            serialFormError("", DEFAULT_BAUD, emptyList(), "", anyPorts = true),
        )
    }

    @Test
    fun `a duplicate name is refused before the invoke rejects it`() {
        assertEquals(
            "A link with that name already exists.",
            serialFormError("/dev/ttyUSB0", DEFAULT_BAUD, listOf("Serial ttyUSB0"), "", anyPorts = true),
        )
    }

    @Test
    fun `a good serial form has nothing to say`() {
        assertNull(serialFormError("/dev/ttyUSB0", DEFAULT_BAUD, listOf("UDP 14550"), "", anyPorts = true))
    }

    @Test
    fun `the automatic name is the port's leaf`() {
        assertEquals("Serial ttyUSB0", autoSerialName("/dev/ttyUSB0"))
    }

    @Test
    fun `an empty port list explains itself rather than blaming the operator`() {
        assertEquals(
            "Nothing is plugged in. Connect a radio over USB and it will appear here.",
            serialFormError("", DEFAULT_BAUD, emptyList(), "", anyPorts = false),
        )
    }
}

class LinkEditRulesTest {
    private fun row(connected: Boolean = false, editing: String = "portOnly") = LinkRow(
        index = 0, name = "UDP 14550", statusLine = "", connected = connected,
        heard = false, lastError = "", editing = editing,
    )

    @Test
    fun `a connected link is not edited underneath itself`() {
        assertFalse(linkIsEditable(row(connected = true)))
    }

    @Test
    fun `a disconnected udp link is editable`() {
        assertTrue(linkIsEditable(row()))
    }

    @Test
    fun `a kind the core has no form for is not editable`() {
        assertFalse(linkIsEditable(row(editing = "none")))
        assertFalse(linkIsEditable(row(editing = "logFile")))
    }

    @Test
    fun `udp writes its own port property and not tcp's`() {
        val writes = editWrites("portOnly", "n", "", 14551, "", 0)
        assertEquals(listOf("name" to "n", "localPort" to 14551), writes)
    }

    @Test
    fun `tcp writes host and port`() {
        val writes = editWrites("hostAndPort", "n", "1.2.3.4", 5760, "", 0)
        assertEquals(listOf<Pair<String, Any>>("name" to "n", "host" to "1.2.3.4", "port" to 5760), writes)
    }

    @Test
    fun `serial writes the port name and baud`() {
        val writes = editWrites("serial", "n", "", 0, "/dev/ttyUSB0", 57600)
        assertEquals(listOf<Pair<String, Any>>("name" to "n", "portName" to "/dev/ttyUSB0", "baud" to 57600), writes)
    }

    @Test
    fun `an unknown kind still renames and writes nothing else`() {
        assertEquals(listOf<Pair<String, Any>>("name" to "n"), editWrites("none", "n", "h", 1, "p", 2))
    }
}

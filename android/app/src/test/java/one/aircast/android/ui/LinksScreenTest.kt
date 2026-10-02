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
    fun `a udp link's server addresses and a serial link's framing are read from the row`() {
        val udp = """{"index":4,"name":"U","statusLine":"","connected":false,"heardVehicle":false,"lastError":"","hostList":["10.0.0.2:14550"]}"""
        val serial = """{"index":5,"name":"S","statusLine":"","connected":false,"heardVehicle":false,"lastError":"","dataBits":7,"stopBits":2,"parity":3,"flowControl":1}"""
        val rows = linkRows(view(udp, serial))
        assertEquals(listOf("10.0.0.2:14550"), rows[0].servers)
        assertEquals(SerialFraming(dataBits = 7, stopBits = 2, parity = 3, flowControl = 1), rows[1].framing)
        assertEquals(SerialFraming(), linkRows(view(heardTcp))[0].framing)
    }

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
        assertEquals("UDP 14550", autoLinkName("udp", "10.0.0.4", "14550"))
        assertEquals("TCP 10.0.0.4:5760", autoLinkName("tcp", "10.0.0.4", "5760"))
        assertEquals("TCP", autoLinkName("tcp", "", "5760"))
    }

    @Test
    fun `a suggested name already in use gets a number, like LinkSettings _uniqueName`() {
        assertEquals("UDP 14550", uniqueLinkName("UDP 14550", listOf("TCP")))
        assertEquals("UDP 14550 (2)", uniqueLinkName("UDP 14550", listOf("UDP 14550")))
        assertEquals("UDP 14550 (3)", uniqueLinkName("UDP 14550", listOf("UDP 14550", "UDP 14550 (2)")))
    }

    @Test
    fun `a port outside the valid range is refused`() {
        assertEquals("Enter a port between 1 and 65535, or leave it blank for 14550", linkFormError("udp", "", "0"))
        assertEquals("Enter a port between 1 and 65535, or leave it blank for 14550", linkFormError("udp", "", "70000"))
        assertEquals("Enter a port between 1 and 65535, or leave it blank for 14550", linkFormError("udp", "", "abc"))
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
    fun `a typed duplicate name is refused, a blank one is numbered instead`() {
        assertEquals(
            "A link with that name already exists.",
            serialFormError("/dev/ttyUSB0", DEFAULT_BAUD, listOf("Serial ttyUSB0"), "Serial ttyUSB0", anyPorts = true),
        )
        assertNull(serialFormError("/dev/ttyUSB0", DEFAULT_BAUD, listOf("Serial ttyUSB0"), "", anyPorts = true))
    }

    @Test
    fun `a good serial form has nothing to say`() {
        assertNull(serialFormError("/dev/ttyUSB0", DEFAULT_BAUD, listOf("UDP 14550"), "", anyPorts = true))
    }

    @Test
    fun `the automatic name is the port's leaf`() {
        assertEquals("Holybro radio (ttyUSB0)", autoSerialName("Holybro radio (ttyUSB0)"))
        assertEquals("a port with no display name is suggested as plain Serial, as SerialSettings does", "Serial", autoSerialName(""))
        assertEquals("5760", portFor("tcp", "14550"))
        assertEquals("14551", portFor("udp", "14551"))
        assertNull("a blank UDP port means the configured listen port", linkFormError("udp", "", "", "14551"))
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
        val writes = editWrites("portOnly", "n", "", 14551, "", 0, false, false)
        assertEquals(listOf<Pair<String, Any>>("name" to "n", "autoConnect" to false, "highLatency" to false, "localPort" to 14551), writes)
    }

    @Test
    fun `tcp writes host and port`() {
        val writes = editWrites("hostAndPort", "n", "1.2.3.4", 5760, "", 0, true, false)
        assertEquals(listOf<Pair<String, Any>>("name" to "n", "autoConnect" to true, "highLatency" to false, "host" to "1.2.3.4", "port" to 5760), writes)
    }

    @Test
    fun `serial writes the port name, baud and the advanced framing`() {
        val writes = editWrites("serial", "n", "", 0, "/dev/ttyUSB0", 57600, false, true, SerialFraming(dataBits = 7, stopBits = 2, parity = 2, flowControl = 1))
        assertEquals(
            listOf<Pair<String, Any>>(
                "name" to "n", "autoConnect" to false, "highLatency" to true, "portName" to "/dev/ttyUSB0", "baud" to 57600,
                "dataBits" to 7, "stopBits" to 2, "parity" to 2, "flowControl" to 1,
            ),
            writes,
        )
    }

    @Test
    fun `an unknown kind still renames and writes nothing else`() {
        assertEquals(listOf<Pair<String, Any>>("name" to "n", "autoConnect" to false, "highLatency" to false), editWrites("none", "n", "h", 1, "p", 2, false, false))
    }

    @Test
    fun `a new link's flags are written to its row`() {
        assertEquals(listOf("links.linkConfigurations.3.autoConnect" to true, "links.linkConfigurations.3.highLatency" to false), linkFlagWrites(3, true, false))
    }

    @Test
    fun `a row reads its connect-on-start and high latency flags`() {
        val rows = linkRows(org.json.JSONObject("""{"configured":[{"index":0,"name":"n","autoConnect":true,"highLatency":true}]}"""))
        assertEquals(true to true, rows[0].autoConnect to rows[0].highLatency)
    }
}

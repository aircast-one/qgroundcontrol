package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class LinkDraftTest {
    private val plugged = listOf(SerialPortChoice("/dev/ttyUSB0", "Holybro radio (ttyUSB0)"))

    @Test
    fun `a link that saved but could not connect counts as added, not as a failure`() {
        assertEquals(AddOutcome.Connected, addOutcome(created = true, saved = true))
        assertEquals(AddOutcome.SavedNotConnected, addOutcome(created = false, saved = true))
        assertEquals(AddOutcome.Failed, addOutcome(created = false, saved = false))
    }

    @Test
    fun `only a link that was never saved blames the name`() {
        assertEquals("Could not add that link. The name may already be in use.", addFailure(LinkType.Tcp))
        assertEquals("Could not start the simulated vehicle.", addFailure(LinkType.Mock))
    }

    @Test
    fun `each kind of new link asks for what it cannot work without`() {
        assertEquals("A TCP link needs the address of the device to call.", draftError(LinkDraft(LinkType.Tcp, port = "5760"), "14550", emptyList(), anyPorts = false))
        assertNull(draftError(LinkDraft(LinkType.Tcp, host = "10.0.0.2", port = "5760"), "14550", emptyList(), anyPorts = false))
        assertNull(draftError(LinkDraft(LinkType.Udp), "14550", emptyList(), anyPorts = false))
        assertEquals("Pick a Bluetooth device.", draftError(LinkDraft(LinkType.Bluetooth), "14550", emptyList(), anyPorts = false))
        assertEquals("Choose a log file to replay.", draftError(LinkDraft(LinkType.LogReplay), "14550", emptyList(), anyPorts = false))
        assertEquals("Pick the port the radio is plugged into.", draftError(LinkDraft(LinkType.Serial), "14550", emptyList(), anyPorts = true))
        assertEquals("", draftError(LinkDraft(LinkType.AircastCloud), "14550", emptyList(), anyPorts = false))
        assertNull(draftError(LinkDraft(LinkType.Mock), "14550", emptyList(), anyPorts = false))
    }

    @Test
    fun `a single plugged-in radio is picked for you, and a choice survives another radio appearing`() {
        assertEquals("/dev/ttyUSB0", LinkDraft(LinkType.Serial).withPluggedPort(plugged).portName)
        assertEquals("", LinkDraft(LinkType.Serial).withPluggedPort(plugged + SerialPortChoice("/dev/ttyACM0", "Pixhawk")).portName)
        assertEquals("/dev/ttyACM0", LinkDraft(LinkType.Serial, portName = "/dev/ttyACM0").withPluggedPort(plugged).portName)
    }

    @Test
    fun `a new link is named after what it points at`() {
        assertEquals("UDP 14550", suggestedLinkName(LinkDraft(LinkType.Udp), emptyList(), "14550"))
        assertEquals("TCP 10.0.0.2:5760", suggestedLinkName(LinkDraft(LinkType.Tcp, host = "10.0.0.2"), emptyList(), "14550"))
        assertEquals("Holybro radio (ttyUSB0)", suggestedLinkName(LinkDraft(LinkType.Serial, portName = "/dev/ttyUSB0"), plugged, "14550"))
        assertEquals(REPLAY_LINK_NAME, suggestedLinkName(LinkDraft(LinkType.LogReplay), emptyList(), "14550"))
    }

    @Test
    fun `link kinds and edit forms round-trip the core's strings, and anything new reads as Other or None`() {
        assertEquals(LinkType.entries.filter { it != LinkType.Other }, LinkType.entries.filter { it != LinkType.Other }.map { LinkType.from(it.id) })
        assertEquals(LinkType.Other, LinkType.from("satellite"))
        assertEquals(LinkType.Other, LinkType.from(""))
        assertEquals(LinkEditing.PortOnly, LinkEditing.from("portOnly"))
        assertEquals(LinkEditing.None, LinkEditing.from(""))
    }
}

class LinkEditTest {
    private fun row(
        editing: LinkEditing = LinkEditing.PortOnly,
        autoConnect: Boolean = false,
        servers: List<String> = emptyList(),
        framing: SerialFraming = SerialFraming(),
    ) = LinkRow(
        index = 0, name = "UDP 14550", statusLine = "", connected = false, heard = false, lastError = "",
        editing = editing, port = 14550, autoConnect = autoConnect, servers = servers, framing = framing,
    )

    @Test
    fun `edit opens Advanced when the link already uses something in it`() {
        assertFalse(editShowsAdvanced(row()))
        assertTrue(editShowsAdvanced(row(autoConnect = true)))
        assertTrue(editShowsAdvanced(row(servers = listOf("10.0.0.9:14550"))))
        assertTrue(editShowsAdvanced(row(editing = LinkEditing.Serial, framing = SerialFraming(dataBits = 7))))
    }

    @Test
    fun `edit checks only the field that link kind has`() {
        assertNull(editError(LinkEditing.PortOnly, linkEdit(row()), "14550"))
        assertEquals("Enter a port between 1 and 65535, or leave it blank for 14550", editError(LinkEditing.PortOnly, linkEdit(row()).copy(port = "0"), "14550"))
        assertEquals("Port must be a number between 1 and 65535.", editError(LinkEditing.HostAndPort, linkEdit(row()).copy(port = "abc"), "14550"))
        assertEquals("Choose a log file.", editError(LinkEditing.LogFile, linkEdit(row()).copy(logFile = ""), "14550"))
        assertNull(editError(LinkEditing.Serial, linkEdit(row()), "14550"))
    }

    @Test
    fun `a blank udp port saves as the configured listen port`() {
        assertEquals(14551, editedPort(LinkEditing.PortOnly, "", "14551"))
        assertEquals(0, editedPort(LinkEditing.HostAndPort, "", "14551"))
    }

    @Test
    fun `a link whose address is wrong offers editing first and a retry second`() {
        assertEquals("Disconnect", primaryLinkAction(connected = true, fixByEditing = false))
        assertEquals("Try again", primaryLinkAction(connected = false, fixByEditing = true))
        assertEquals("Connect", primaryLinkAction(connected = false, fixByEditing = false))
    }

    @Test
    fun `the settings sheet hides its tabs inside a page or aircraft setup`() {
        assertFalse(sheetDrilled(openPage = null, setupOpen = false))
        assertTrue(sheetDrilled(openPage = "Connections", setupOpen = false))
        assertTrue(sheetDrilled(openPage = null, setupOpen = true))
    }
}

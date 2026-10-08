package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class AddableLinkTypesTest {

    private fun links(ids: String) = JSONObject(
        """{"kind":"object","class":"Links","links":[],"linkTypeIds":$ids}""",
    )

    @Test
    fun `a build without serial support stops offering serial`() {
        assertEquals(
            "linkTypeTable() wraps serial in QGC_NO_SERIAL_LINK, and createSerialConfiguration " +
                "returns false on such a build - the chip was hardcoded, so it offered a link type " +
                "that could only ever fail",
            listOf(LinkType.Udp, LinkType.Tcp),
            addableLinkTypes(links("""["udp","tcp"]""")),
        )
    }

    @Test
    fun `a type this head cannot create is not offered just because the core lists it`() {
        assertEquals(
            "createAndConnectLink handles udp and tcp and returns false for anything else, and " +
                "Bluetooth is only offered where the core has a Bluetooth host",
            listOf(LinkType.Udp, LinkType.Tcp, LinkType.Serial),
            addableLinkTypes(links("""["serial","udp","tcp","bluetooth"]""")),
        )
    }

    @Test
    fun `a core too old to serve the list keeps all three rather than offering none`() {
        assertEquals(CREATABLE_LINK_TYPES, addableLinkTypes(JSONObject("""{"kind":"object"}""")))
        assertEquals(CREATABLE_LINK_TYPES, addableLinkTypes(null))
        assertEquals(CREATABLE_LINK_TYPES, addableLinkTypes(links("[]")))
    }

    @Test
    fun `bluetooth is offered when the core has a bluetooth host`() {
        val view = JSONObject("""{"kind":"object","linkTypeIds":["serial","udp","tcp","bluetooth"],
            "bluetooth":{"available":true,"scanning":false,"devices":[{"name":"HC-05","address":"98:D3:31:F6:12:34"}]}}""")
        assertEquals(listOf(LinkType.Udp, LinkType.Tcp, LinkType.Serial, LinkType.Bluetooth), addableLinkTypes(view))
        assertEquals(listOf(BluetoothDeviceChoice("HC-05", "98:D3:31:F6:12:34")), bluetoothState(view).devices)
    }

    @Test
    fun `log replay is offered when the core lists it, after the live links`() {
        assertEquals(
            listOf(LinkType.Udp, LinkType.Tcp, LinkType.Serial, LinkType.LogReplay),
            addableLinkTypes(links("""["serial","udp","tcp","logReplay"]""")),
        )
    }

    @Test
    fun `each type is named in full, as LinkManager's type strings are`() {
        assertEquals(
            listOf("UDP", "TCP", "Serial", "Bluetooth", "Log replay", "Aircast Cloud"),
            listOf(LinkType.Udp, LinkType.Tcp, LinkType.Serial, LinkType.Bluetooth, LinkType.LogReplay, LinkType.AircastCloud).map(::linkTypeLabel),
        )
    }

    @Test
    fun `add link lists pilot links first, puts a plugged-in USB radio on top, and keeps replay and simulation apart`() {
        val offered = listOf(LinkType.Udp, LinkType.Tcp, LinkType.Serial, LinkType.Bluetooth, LinkType.LogReplay, LinkType.Mock, LinkType.AircastCloud)
        assertEquals(listOf(LinkType.Udp, LinkType.Serial, LinkType.Bluetooth, LinkType.AircastCloud, LinkType.Tcp), pilotLinkKinds(offered, radioPlugged = false))
        assertEquals(listOf(LinkType.Serial, LinkType.Udp, LinkType.Bluetooth, LinkType.AircastCloud, LinkType.Tcp), pilotLinkKinds(offered, radioPlugged = true))
        assertEquals(listOf(LinkType.LogReplay, LinkType.Mock), toolLinkKinds(offered))
        assertEquals(listOf(LinkType.Udp), pilotLinkKinds(listOf(LinkType.Udp, LinkType.LogReplay), radioPlugged = true))
    }

    @Test
    fun `every link kind has a pilot title, an explanation, and a subtitle within the 32-character row budget`() {
        (PILOT_LINK_ORDER + TOOL_LINK_ORDER).map(::linkKind).map { kind ->
            org.junit.Assert.assertTrue(kind.type.id, kind.title.isNotBlank() && kind.about.isNotBlank() && kind.detail.length <= 32)
        }
    }
}

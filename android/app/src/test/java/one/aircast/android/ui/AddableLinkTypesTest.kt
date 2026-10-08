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
            listOf("udp", "tcp"),
            addableLinkTypes(links("""["udp","tcp"]""")),
        )
    }

    @Test
    fun `a type this head cannot create is not offered just because the core lists it`() {
        assertEquals(
            "createAndConnectLink handles udp and tcp and returns false for anything else, and " +
                "Bluetooth is only offered where the core has a Bluetooth host",
            listOf("udp", "tcp", "serial"),
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
        assertEquals(listOf("udp", "tcp", "serial", "bluetooth"), addableLinkTypes(view))
        assertEquals(listOf(BluetoothDeviceChoice("HC-05", "98:D3:31:F6:12:34")), bluetoothState(view).devices)
    }

    @Test
    fun `log replay is offered when the core lists it, after the live links`() {
        assertEquals(
            listOf("udp", "tcp", "serial", REPLAY_LINK),
            addableLinkTypes(links("""["serial","udp","tcp","logReplay"]""")),
        )
    }

    @Test
    fun `each type is named in full, as LinkManager's type strings are`() {
        assertEquals(
            listOf("UDP", "TCP", "Serial", "Bluetooth", "Log replay", "Aircast Cloud"),
            listOf("udp", "tcp", "serial", BLUETOOTH_LINK, REPLAY_LINK, AIRCAST_CLOUD_LINK).map(::linkTypeLabel),
        )
    }

    @Test
    fun `add link lists pilot links first, puts a plugged-in USB radio on top, and keeps replay and simulation apart`() {
        val offered = listOf("udp", "tcp", "serial", BLUETOOTH_LINK, REPLAY_LINK, MOCK_LINK, AIRCAST_CLOUD_LINK)
        assertEquals(listOf("udp", "serial", BLUETOOTH_LINK, AIRCAST_CLOUD_LINK, "tcp"), pilotLinkKinds(offered, radioPlugged = false))
        assertEquals(listOf("serial", "udp", BLUETOOTH_LINK, AIRCAST_CLOUD_LINK, "tcp"), pilotLinkKinds(offered, radioPlugged = true))
        assertEquals(listOf(REPLAY_LINK, MOCK_LINK), toolLinkKinds(offered))
        assertEquals(listOf("udp"), pilotLinkKinds(listOf("udp", REPLAY_LINK), radioPlugged = true))
    }

    @Test
    fun `every link kind reads in pilot words with a short line that fits one row`() {
        (PILOT_LINK_ORDER + TOOL_LINK_ORDER).map(::linkKind).map { kind ->
            org.junit.Assert.assertTrue(kind.id, kind.title.isNotBlank() && kind.about.isNotBlank() && kind.detail.length <= 32)
        }
    }
}

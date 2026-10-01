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
            addableLinkTypes(links("""["serial","udp","tcp","bluetooth","mock"]""")),
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
}

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
                "there is no Bluetooth creation path in the bridge at all - listing it would draw " +
                "a chip that cannot make a link",
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
}

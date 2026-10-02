package one.aircast.android.ui

import one.aircast.android.bridge.Fact
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class RemoteIdIndicatorTest {
    @Test
    fun `rows follow the indicator page`() {
        val status = remoteIdStatus(
            JSONObject("""{"shown":true,"state":"warning","comms":true,"armStatus":true,"gcsGps":false,"basicId":true,"operatorIdShown":false,"operatorId":false,"emergency":false,"emergencyHoldMs":800}"""),
        )!!
        assertEquals(listOf("RID COMMS", "ARM STATUS", "GCS GPS", "BASIC ID"), remoteIdRows(status).map { it.first })
        assertEquals(800L, status.holdMs)
        val offline = status.copy(comms = false)
        assertEquals(listOf("NOT CONNECTED" to false), remoteIdRows(offline))
        assertNull(remoteIdStatus(JSONObject("""{"shown":false}""")))
    }

    private fun setting(name: String, value: String) = Fact(
        path = "settings.remoteIDSettings.$name", name = name, description = "", units = "", valueString = value, value = value,
        enumStrings = emptyList(), enumIndex = -1, isBool = false, isString = true, readOnly = false,
    )

    @Test
    fun `self id lists QGC's facts in order and greys the message fields while broadcast is off`() {
        val page = listOf(setting("selfIDFree", "x"), setting("region", "0"), setting("selfIDEmergency", "help"), setting("sendSelfID", "false"), setting("selfIDType", "0"))
        val off = selfIdFacts(page)
        assertEquals(listOf("sendSelfID", "selfIDType", "selfIDFree", "selfIDEmergency"), off.map { it.name })
        assertEquals(listOf(true, false, false, true), off.map { it.acceptsWrite })
        assertEquals(listOf(true, true, true, true), selfIdFacts(page.map { if (it.name == "sendSelfID") it.copy(valueString = "true", value = true) else it }).map { it.acceptsWrite })
    }
}

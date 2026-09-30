package one.aircast.android.ui

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
}

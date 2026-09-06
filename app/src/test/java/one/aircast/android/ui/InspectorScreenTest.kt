package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class InspectorScreenTest {
    @Test
    fun `messages are parsed and sorted by name`() {
        val model = JSONObject(
            """
            {"count":2,"elements":[
              {"id":30,"name":"ATTITUDE","actualRateHz":10.0,"count":49},
              {"id":0,"name":"AHRS","actualRateHz":3.0,"count":15}
            ]}
            """.trimIndent(),
        )
        val messages = parseInspectorMessages(model)
        assertEquals(listOf("AHRS", "ATTITUDE"), messages.map { it.name })
        assertEquals(1, messages[0].index)
        assertEquals(0, messages[1].index)
        assertEquals(10.0, messages[1].rateHz, 1e-9)
    }

    @Test
    fun `an absent message model yields nothing`() {
        assertEquals(emptyList<InspectorMessage>(), parseInspectorMessages(null))
        assertEquals(emptyList<InspectorMessage>(), parseInspectorMessages(JSONObject("{}")))
    }

    @Test
    fun `fields keep their declared order`() {
        val model = JSONObject(
            """
            {"elements":[
              {"name":"roll","type":"float","value":"0.001"},
              {"name":"pitch","type":"float","value":"0.002"}
            ]}
            """.trimIndent(),
        )
        val fields = parseInspectorFields(model)
        assertEquals(listOf("roll", "pitch"), fields.map { it.name })
        assertEquals(InspectorField("roll", "float", "0.001"), fields[0])
    }

    @Test
    fun `rate formatting is fixed to one decimal`() {
        assertEquals("10.0 Hz", formatRate(10.0))
        assertEquals("3.5 Hz", formatRate(3.46))
        assertEquals("0.0 Hz", formatRate(0.0))
    }

    @Test
    fun `an unknown rate reads as unavailable`() {
        assertEquals("--", formatRate(Double.NaN))
    }
}

package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class GcsPositionStatusTest {

    @Test
    fun `rows read as GcsPositionStatus does and hide without a valid position`() {
        val position = JSONObject("""{"kind":"value","value":{"latitude":47.123456789,"longitude":8.5,"valid":true}}""")
        assertEquals(
            listOf("Latitude" to "47.1234568", "Longitude" to "8.5000000", "HDOP" to "2.3 m"),
            gcsPositionRows(position, JSONObject("""{"value":2.345}""")),
        )
        assertEquals("N/A", gcsPositionRows(position, JSONObject("""{"value":0}"""))!!.last().second)
        assertNull(gcsPositionRows(JSONObject("""{"value":{"valid":false}}"""), null))
        assertNull(gcsPositionRows(null, null))
    }
}

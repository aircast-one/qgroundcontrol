package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ProximityRadarTest {
    @Test
    fun `the radar shows only while distance telemetry arrives`() {
        assertNull(proximityRadar(JSONObject("""{"shown":false,"sectors":[]}""")))
        val radar = proximityRadar(JSONObject("""{"shown":true,"rangeMeters":6,"sectors":[{"bearing":0,"meters":2.5,"text":"2.50"},{"bearing":45,"meters":null,"text":"–.––"}]}"""))!!
        assertEquals(6.0, radar.range, 0.0)
        assertEquals(listOf(RadarSector(0, 2.5, "2.50"), RadarSector(45, null, "–.––")), radar.sectors)
    }
}

package one.aircast.map

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class RoiMarkerTest {
    @Test
    fun `the marker shows only while the vehicle reports an active roi`() {
        assertEquals(TrackPoint(47.4, 8.5), roiPoint(JSONObject("""{"roiActive":true,"roi":{"latitude":47.4,"longitude":8.5}}""")))
        assertNull(roiPoint(JSONObject("""{"roiActive":false,"roi":null}""")))
        assertNull(roiPoint(JSONObject("""{"roi":{"latitude":0.0,"longitude":0.0}}""")))
        assertNull(roiPoint(null))
    }
}

package one.aircast.map

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class AutodropTargetMarkerTest {
    @Test
    fun `the marker reads the value envelope the fact system wraps it in`() {
        assertEquals(TrackPoint(47.4, 8.5), autodropTargetPoint(JSONObject("""{"kind":"value","value":{"latitude":47.4,"longitude":8.5}}""")))
        assertNull(autodropTargetPoint(JSONObject("""{"kind":"value","value":null}""")))
        assertNull(autodropTargetPoint(JSONObject("""{"kind":"value","value":{"latitude":0.0,"longitude":0.0}}""")))
        assertNull(autodropTargetPoint(null))
    }
}

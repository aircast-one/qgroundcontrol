package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class GeoTagViewTest {
    @Test
    fun `the path carries the log it is asking about`() {
        assertEquals("view.geoTag(/a/b.tlog)", geoTagPath("/a/b.tlog"))
    }

    @Test
    fun `a served reading is parsed, and anything else is not a reading`() {
        val view = JSONObject("""{"class":"GeoTag","readable":true,"bytes":163381,"triggerCount":4,"refusal":"noImages"}""")

        assertEquals(GeoTagReading(true, 163381, 4, "noImages"), geoTagReading(view))
        assertNull(geoTagReading(JSONObject("""{"kind":"null"}""")))
        assertNull(geoTagReading(null))
    }

    @Test
    fun `a log with no triggers says why that matters`() {
        assertEquals(
            "No camera triggers were recorded, so there is nothing to match photographs against.",
            triggerSummary(GeoTagReading(true, 100, 0, "noImages")),
        )
        assertEquals("1 camera trigger recorded.", triggerSummary(GeoTagReading(true, 100, 1, "")))
        assertEquals("12 camera triggers recorded.", triggerSummary(GeoTagReading(true, 100, 12, "")))
        assertEquals("This file could not be read.", triggerSummary(GeoTagReading(false, 0, 0, "")))
    }

    @Test
    fun `sizes read in the unit that fits`() {
        assertEquals("163 KB", logSize(167000))
        assertEquals("1.6 MB", logSize(1_700_000))
        assertEquals("400 B", logSize(400))
    }
}

class RefusalTest {
    @Test
    fun `an accepted command has nothing to say`() {
        assertNull(one.aircast.android.bridge.refusal(JSONObject("""{"ok":true,"mode":1}""")))
    }

    @Test
    fun `a refusal is the sentence the core wrote`() {
        assertEquals(
            "The camera is still taking the last photo.",
            one.aircast.android.bridge.refusal(
                JSONObject("""{"ok":false,"reason":"The camera is still taking the last photo."}"""),
            ),
        )
    }

    @Test
    fun `a refusal with no sentence, and no answer at all, still say something`() {
        assertEquals("The vehicle refused.", one.aircast.android.bridge.refusal(JSONObject("""{"ok":false}""")))
        assertEquals("The vehicle did not answer.", one.aircast.android.bridge.refusal(null))
    }
}

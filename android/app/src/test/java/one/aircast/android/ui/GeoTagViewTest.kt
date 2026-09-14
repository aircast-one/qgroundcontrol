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

class SensorHealthTest {
    private fun view(json: String) = sensorHealth(JSONObject(json))

    @Test
    fun `a healthy vehicle says nothing extra`() {
        val read = view("""{"class":"SensorHealth","available":true,"failing":[],"status":"","sensors":[{"name":"GPS","state":"healthy","label":"Healthy"}]}""")

        assertEquals(1, read!!.sensors.size)
        assertEquals(SensorHealth("GPS", "healthy", "Healthy"), read.sensors.first())
        assertEquals("", healthSummary(read))
    }

    @Test
    fun `a fault is named, and several are counted`() {
        val one = view("""{"class":"SensorHealth","available":true,"failing":["Magnetometer"],"sensors":[]}""")
        val two = view("""{"class":"SensorHealth","available":true,"failing":["Magnetometer","Gyro"],"sensors":[]}""")

        assertEquals("Magnetometer is reporting a fault.", healthSummary(one))
        assertEquals("2 sensors are reporting faults.", healthSummary(two))
    }

    @Test
    fun `anything that is not the sensor health view is not a reading`() {
        assertNull(sensorHealth(JSONObject("""{"kind":"null"}""")))
        assertNull(sensorHealth(null))
        assertEquals("", healthSummary(null))
    }
}

class GeoTagCommaTest {
    @Test
    fun `a path the core can parse becomes a view argument`() {
        assertEquals("view.geoTag(/a/b.tlog)", geoTagPath("/a/b.tlog"))
        assertEquals("view.geoTag(/Aircast QGC Daily/b.tlog)", geoTagPath("/Aircast QGC Daily/b.tlog"))
    }

    @Test
    fun `a comma is refused rather than sent to be truncated`() {
        assertNull(geoTagPath("/Flights, 2026/b.tlog"))
        assertEquals(
            "This log cannot be read: a comma in its folder name is not something the core can tell from a tolerance.",
            commaRefusal(TelemetryLog("/Flights, 2026/b.tlog", "b.tlog", 10)),
        )
        assertNull(commaRefusal(TelemetryLog("/a/b.tlog", "b.tlog", 10)))
    }
}

package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class SensorSettingsTest {
    @Test
    fun compasses_and_priority_slots_read_from_the_core() {
        assertNull(sensorSettings(JSONObject("""{"available":false}""")))
        val read = sensorSettings(JSONObject("""{"available":true,"boardRotation":null,"priorities":["Priority 1","Priority 2","Priority 3","Not Set"],
            "compasses":[{"index":0,"label":"Compass 1 (primary, external)","device":"IST8310 (I2C1)","use":null,"priority":0,"orientation":null}],
            "helpSet":"a","helpCal":"b","simpleAccelHelp":"c","declination":null}"""))!!
        assertEquals(CompassSettings(0, "Compass 1 (primary, external)", "IST8310 (I2C1)", null, 0, null), read.compasses.single())
        assertEquals("Not Set", read.priorities[3])
    }
}

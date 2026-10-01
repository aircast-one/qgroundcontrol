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
            "compasses":[{"index":0,"label":"Compass 1 (primary, external)","device":"IST8310 (I2C1)","use":null,"priority":0,"orientation":null,"orientationTitle":"Orientation"}],"boardTitle":"Autopilot Rotation","compassesWhileCalibrating":true,
            "helpSet":"a","helpCal":"b","simpleAccelHelp":"c","declination":null}"""))!!
        assertEquals(CompassSettings(0, "Compass 1 (primary, external)", "IST8310 (I2C1)", null, 0, null, "Orientation"), read.compasses.single())
        assertEquals("Not Set", read.priorities[3])
    }

    @Test
    fun px4_mags_carry_their_own_titles_and_stay_out_of_the_calibration_dialog() {
        val read = sensorSettings(JSONObject("""{"available":true,"boardRotation":null,"boardTitle":"Autopilot Orientation","compassesWhileCalibrating":false,"priorities":[],
            "compasses":[{"index":1,"label":"Mag 1","device":"","use":null,"priority":null,"orientation":null,"orientationTitle":"Mag 1 Orientation"}],
            "helpSet":"a","helpCal":"b","simpleAccelHelp":"","declination":null}"""))!!
        assertEquals("Autopilot Orientation", read.boardTitle)
        assertEquals("Mag 1 Orientation", read.compasses.single().orientationTitle)
        assertEquals(false, read.compassesWhileCalibrating)
    }
}

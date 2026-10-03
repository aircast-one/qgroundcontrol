package one.aircast.android.ui

import one.aircast.android.R
import org.junit.Assert.assertEquals
import org.junit.Test

class CalibrationSideTest {
    private fun side(stage: String, rotate: Boolean = false) = CalibrationSide("NoseDown", "Nose down", true, stage, rotate)

    @Test
    fun `each side shows VehicleRotationCal's picture and state`() {
        assertEquals(R.drawable.cal_vehicle_nose_down, sideImage("NoseDown", rotating = false))
        assertEquals(R.drawable.cal_vehicle_nose_down_rotate, sideImage("NoseDown", rotating = true))
        assertEquals(R.drawable.cal_vehicle_down, sideImage("Down", rotating = false))
        assertEquals("Rotate", sideStateText(side("inProgress", rotate = true)))
        assertEquals("Hold still", sideStateText(side("inProgress")))
        assertEquals("Done", sideStateText(side("done")))
        assertEquals("Pending", sideStateText(side("idle")))
    }
}

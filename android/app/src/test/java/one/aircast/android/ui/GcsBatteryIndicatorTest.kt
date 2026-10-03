package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class GcsBatteryIndicatorTest {
    @Test
    fun `the phone battery is asked of the core as a percentage`() {
        assertEquals(PhoneBattery(54, false), phoneBattery(54, 100, 3))
        assertEquals(PhoneBattery(50, true), phoneBattery(128, 256, 2))
        assertNull(phoneBattery(-1, 100, 2))
        assertEquals("view.gcsBattery(54,false)", gcsBatteryPath(PhoneBattery(54, false)))
        val reading = gcsBatteryReading(JSONObject("""{"state":"low","levelText":"20%","stateText":"On battery","heading":"Ground Station","title":"Ground station battery"}"""))!!
        assertEquals("low", reading.state)
        assertEquals("Ground station battery", reading.title)
    }
}

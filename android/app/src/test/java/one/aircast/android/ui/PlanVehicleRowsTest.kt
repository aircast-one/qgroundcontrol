package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class PlanVehicleRowsTest {
    @Test
    fun `the planned vehicle is chosen only with no vehicle and an empty plan`() {
        assertEquals(true, choosesPlanVehicle(false, JSONObject("""{"hasMissionItems":false}""")))
        assertEquals(false, choosesPlanVehicle(true, JSONObject("""{"hasMissionItems":false}""")))
        assertEquals(false, choosesPlanVehicle(false, JSONObject("""{"hasMissionItems":true}""")))
        assertEquals("a fence alone leaves the vehicle choosable, as MissionSettingsEditor counts mission items only", true, choosesPlanVehicle(false, JSONObject("""{"containsItems":true,"hasMissionItems":false}""")))
    }
}

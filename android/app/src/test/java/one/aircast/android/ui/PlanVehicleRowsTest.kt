package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class PlanVehicleRowsTest {
    @Test
    fun `the planned vehicle is chosen only with no vehicle and an empty plan`() {
        assertEquals(true, choosesPlanVehicle(false, JSONObject("""{"containsItems":false}""")))
        assertEquals(false, choosesPlanVehicle(true, JSONObject("""{"containsItems":false}""")))
        assertEquals(false, choosesPlanVehicle(false, JSONObject("""{"containsItems":true}""")))
    }
}

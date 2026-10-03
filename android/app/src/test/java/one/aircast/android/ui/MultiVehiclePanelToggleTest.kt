package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class MultiVehiclePanelToggleTest {
    @Test
    fun `the toggle follows the fact's visible flag, as MultiVehicleSelector does`() {
        assertEquals(false, panelToggleShown(JSONObject("""{"visible":false}""")))
        assertEquals(true, panelToggleShown(JSONObject("""{"visible":true}""")))
        assertEquals(true, panelToggleShown(null))
    }
}

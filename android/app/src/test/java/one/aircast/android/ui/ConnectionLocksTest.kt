package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ConnectionLocksTest {

    @Test
    fun `the locks follow MultiVehicleManager's first vehicle in, last vehicle out`() {
        assertTrue(vehicleConnected(JSONObject("""{"count":1}""")))
        assertTrue(vehicleConnected(JSONObject("""{"count":3}""")))
        assertFalse(vehicleConnected(JSONObject("""{"count":0}""")))
        assertFalse(vehicleConnected(null))
    }
}

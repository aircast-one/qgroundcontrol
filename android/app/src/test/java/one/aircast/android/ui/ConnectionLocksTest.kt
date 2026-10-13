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

    @Test
    fun `a link the pilot opened keeps the app on the network before the vehicle answers, an automatic listener does not`() {
        val opened = JSONObject("""{"configured":[{"index":0,"name":"SITL","connected":true,"heardVehicle":false}],"links":[]}""")
        val closed = JSONObject("""{"configured":[{"index":0,"name":"SITL","connected":false}],"links":[]}""")
        val listening = JSONObject("""{"configured":[],"links":[{"index":1,"name":"UDP","dynamic":true,"connected":true}]}""")
        assertTrue(linkAwaitingVehicle(opened))
        assertFalse(linkAwaitingVehicle(closed))
        assertFalse(linkAwaitingVehicle(listening))
        assertFalse(linkAwaitingVehicle(null))
    }
}

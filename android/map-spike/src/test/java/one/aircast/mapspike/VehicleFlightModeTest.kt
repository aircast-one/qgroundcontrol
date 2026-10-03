package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class VehicleFlightModeTest {
    @Test
    fun `each row carries its own settable modes and writes its own vehicle, like MultiVehicleList's FlightModeMenu`() {
        val view = JSONObject("""{"ambiguous":true,"vehicles":[{"id":1,"index":0,"flightModes":["Loiter","RTL"]},{"id":2,"index":1,"flightModes":[]}]}""")
        val choices = vehicleChoices(view).choices
        assertEquals(listOf("Loiter", "RTL"), choices[0].flightModes)
        assertEquals("vehicles.vehicles.1.flightMode", vehicleFlightModePath(choices[1]))
        assertEquals(emptyList<String>(), choices[1].flightModes)
    }
}

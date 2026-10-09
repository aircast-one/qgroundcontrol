package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class VehicleChangePromptTest {
    @Test
    fun `the plan view asks only when the core raises it, and reads the aircraft's stored route`() {
        assertNull(vehicleChangePrompt(JSONObject("""{"vehicleChangePrompt":null}""")))
        assertEquals(
            VehicleChangePrompt(connected = true, dirty = true, aircraftItems = 12),
            vehicleChangePrompt(JSONObject("""{"vehicleChangePrompt":{"connected":true,"dirty":true,"aircraftItems":12}}""")),
        )
        assertNull(vehicleChangePrompt(JSONObject("""{"vehicleChangePrompt":{"connected":true,"dirty":true,"aircraftItems":null}}"""))?.aircraftItems)
    }

    @Test
    fun `a route the operator drew is kept by the main button, and the aircraft's route is named`() {
        val drawn = promptCopy(VehicleChangePrompt(connected = true, dirty = true, aircraftItems = 12))
        assertEquals(PromptCopy("Aircraft connected", "Keep the route you drew, or replace it with the route stored on the aircraft (12 items)?", "Keep my route", "Load the aircraft's route", primaryKeeps = true), drawn)
        assertEquals(false, promptCopy(VehicleChangePrompt(connected = true, dirty = false, aircraftItems = null)).primaryKeeps)
        assertEquals("Discard it", promptCopy(VehicleChangePrompt(connected = false, dirty = true, aircraftItems = null)).secondary)
    }
}

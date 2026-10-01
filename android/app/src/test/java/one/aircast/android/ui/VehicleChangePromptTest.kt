package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class VehicleChangePromptTest {
    @Test
    fun `the plan view asks about a dirty plan only when the core raises it`() {
        assertNull(vehicleChangePrompt(JSONObject("""{"vehicleChangePrompt":null}""")))
        val prompt = vehicleChangePrompt(JSONObject("""{"vehicleChangePrompt":{"title":"Plan View - Vehicle Changed","text":"t","loadText":"Load New Plan From Vehicle","keepText":"Keep Current Plan"}}"""))
        assertEquals("Keep Current Plan", prompt?.keepText)
    }
}

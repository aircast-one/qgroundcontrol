package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SyntheticViewTest {
    @Test
    fun `the view is drawn only when the core can place the camera`() {
        assertTrue(syntheticAvailable(JSONObject("""{"available":true,"latitude":1.0}""")))
        assertFalse(syntheticAvailable(JSONObject("""{"available":false}""")))
        assertFalse(syntheticAvailable(null))
    }

    @Test
    fun `the page gets the core's pose as it is, and only once it has loaded`() {
        val script = syntheticPoseScript(JSONObject("""{"available":true,"heading":90}"""))
        assertTrue(script.startsWith("window.aircast && window.aircast.pose("))
        assertTrue(script.contains(""""heading":90"""))
    }
}

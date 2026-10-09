package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
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
    fun `dragging the view down looks up and dragging up looks down, a full height being ninety degrees`() {
        assertEquals(-60.0, syntheticTilt(-15.0, draggedPx = -500f, heightPx = 1000), 1e-9)
        assertEquals(-6.0, syntheticTilt(-15.0, draggedPx = 100f, heightPx = 1000), 1e-9)
        assertEquals(0.0, syntheticTilt(-15.0, draggedPx = 900f, heightPx = 1000), 1e-9)
        assertEquals(-90.0, syntheticTilt(-15.0, draggedPx = -2000f, heightPx = 1000), 1e-9)
    }

    @Test
    fun `dragging sideways turns the view so the scene follows the finger`() {
        assertEquals(-35.0, syntheticPan(0.0, draggedPx = 500f, widthPx = 1000, fovDeg = 70.0), 1e-9)
        assertEquals(35.0, syntheticPan(0.0, draggedPx = -500f, widthPx = 1000, fovDeg = 70.0), 1e-9)
        assertEquals(-170.0, syntheticPan(170.0, draggedPx = -2000f, widthPx = 1000, fovDeg = 10.0), 1e-9)
    }

    @Test
    fun `the page asks for tiles beside itself and the core answers them`() {
        assertEquals("/Bing%20Satellite/15/29961/19829", syntheticTilePath("/assets/synthetic/tiles/Bing%20Satellite/15/29961/19829"))
        assertEquals(null, syntheticTilePath("/assets/synthetic/Cesium/Cesium.js"))
        assertEquals(null, syntheticTilePath(null))
    }

    @Test
    fun `the page gets the core's pose as it is, and only once it has loaded`() {
        val script = syntheticPoseScript(JSONObject("""{"available":true,"heading":90}"""))
        assertTrue(script.startsWith("window.aircast && window.aircast.pose("))
        assertTrue(script.contains(""""heading":90"""))
        assertTrue(syntheticOverlaysScript(JSONObject("""{"route":[]}""")).startsWith("window.aircast && window.aircast.overlays("))
    }
}

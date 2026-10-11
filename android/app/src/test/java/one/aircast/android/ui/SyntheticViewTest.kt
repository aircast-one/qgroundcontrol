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
    fun `a full-width swipe turns the view all the way round, as a full-height one covers the whole tilt`() {
        assertEquals(-90.0, syntheticPan(0.0, draggedPx = 250f, widthPx = 1000), 1e-9)
        assertEquals(90.0, syntheticPan(0.0, draggedPx = -250f, widthPx = 1000), 1e-9)
        assertEquals(30.0, syntheticPan(30.0, draggedPx = 1000f, widthPx = 1000), 1e-9)
        assertEquals(-154.0, syntheticPan(170.0, draggedPx = -100f, widthPx = 1000), 1e-9)
    }

    @Test
    fun `the synthetic camera reads live only once the page has drawn it`() {
        val camera = { source: String, status: String -> camerasReading(JSONObject("""{"class":"Cameras","cameras":[{"slot":0,"title":"Synthetic","short":"Synthetic","source":"$source","active":true,"status":"$status"}]}"""))!!.cameras.single() }
        val synthetic = camera(SYNTHETIC_SOURCE, "live")
        assertEquals("a green dot over a black pane told the pilot a blank view was working", CameraStatus.Connecting, shownCameraStatus(synthetic, showing = true, page = SyntheticDraw.Loading))
        assertEquals(CameraStatus.Live, shownCameraStatus(synthetic, showing = true, page = SyntheticDraw.Drawn))
        assertEquals("a page that failed to start is no picture, not one on its way", CameraStatus.NoSignal, shownCameraStatus(synthetic, showing = true, page = SyntheticDraw.Failed))
        assertEquals("a synthetic view that is not on screen has no page to wait for, so it keeps the core's word", CameraStatus.Live, shownCameraStatus(synthetic, showing = false, page = SyntheticDraw.Loading))
        assertEquals(CameraStatus.Idle, shownCameraStatus(camera(SYNTHETIC_SOURCE, "idle"), showing = true, page = SyntheticDraw.Failed))
        assertEquals(CameraStatus.Live, shownCameraStatus(camera("RTSP Video Stream", "live"), showing = true, page = SyntheticDraw.Loading))
    }

    @Test
    fun `any page that has drawn wins, and a failure shows only when none has`() {
        assertEquals(SyntheticDraw.Drawn, pageOf(listOf(SyntheticDraw.Failed, SyntheticDraw.Drawn)))
        assertEquals(SyntheticDraw.Failed, pageOf(listOf(SyntheticDraw.Loading, SyntheticDraw.Failed)))
        assertEquals(SyntheticDraw.Loading, pageOf(emptyList()))
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

    @Test
    fun `sideways drags aim only a synthetic view picked as the camera, not one filling in for a dropped stream`() {
        val aimable = JSONObject("""{"available":true,"aimable":true}""")
        val cameras = { source: String -> camerasReading(JSONObject("""{"class":"Cameras","cameras":[{"slot":0,"source":"$source","active":true},{"slot":1,"source":"RTSP Video Stream","active":false}]}""")) }
        assertTrue(syntheticAims(aimable, cameras(SYNTHETIC_SOURCE)))
        assertFalse("a dropped RTSP stream shows the synthetic view as a fallback, and a swipe still switches cameras", syntheticAims(aimable, cameras("RTSP Video Stream")))
        assertFalse(syntheticAims(JSONObject("""{"available":true,"aimable":false}"""), cameras(SYNTHETIC_SOURCE)))
        assertFalse(syntheticAims(aimable, null))
    }
}

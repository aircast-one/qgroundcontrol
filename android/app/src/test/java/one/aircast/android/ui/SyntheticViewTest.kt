package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class SyntheticViewTest {
    @Test
    fun `the synthetic view says which camera it stands in for once it has drawn, then steps off the aim point to the edge`() {
        val standIn = "No video from SITL camera · showing synthetic view"
        assertEquals("an empty pane has no aim point to cover", ShownNote("Loading 3D view…", atEdge = false), shownNote(SyntheticDraw.Loading, standIn, announced = true))
        assertEquals(ShownNote("3D view failed to start", atEdge = false), shownNote(SyntheticDraw.Failed, standIn, announced = true))
        assertEquals("the fallback is announced in the middle first", ShownNote(standIn, atEdge = false), shownNote(SyntheticDraw.Drawn, standIn, announced = false))
        assertEquals("once announced it leaves the middle, where the camera points, clear", ShownNote(standIn, atEdge = true), shownNote(SyntheticDraw.Drawn, standIn, announced = true))
        assertNull("the synthetic view chosen as the camera says nothing", shownNote(SyntheticDraw.Drawn, null, announced = false))
    }

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
        assertEquals(90.0, syntheticPan(0.0, draggedPx = 250f, widthPx = 1000), 1e-9)
        assertEquals(-90.0, syntheticPan(0.0, draggedPx = -250f, widthPx = 1000), 1e-9)
        assertEquals(30.0, syntheticPan(30.0, draggedPx = 1000f, widthPx = 1000), 1e-9)
        assertEquals(134.0, syntheticPan(170.0, draggedPx = -100f, widthPx = 1000), 1e-9)
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
    fun `with a gimbal aiming the view a finger drag steers the gimbal, without one it looks around`() {
        val gimballed = JSONObject("""{"available":true,"aimable":false}""")
        assertTrue("the view follows the gimbal, so a drag has to move the gimbal or it does nothing", gimbalAimsSynthetic(gimballed, aimable = true))
        assertFalse("free look keeps its own drag", gimbalAimsSynthetic(JSONObject("""{"available":true,"aimable":true}"""), aimable = true))
        assertFalse("a thumbnail or second pane never takes the drag", gimbalAimsSynthetic(gimballed, aimable = false))
        assertFalse(gimbalAimsSynthetic(JSONObject("""{"available":false,"aimable":false}"""), aimable = true))
    }

    @Test
    fun `a drag starts from where the active gimbal points`() {
        val view = JSONObject("""{"gimbals":[{"active":false,"pitch":-80.0,"bodyYaw":90.0},{"active":true,"pitch":-12.5,"bodyYaw":30.0}]}""")
        assertEquals(-12.5 to 30.0, gimbalAim(view))
        assertEquals("a gimbal that has not reported its attitude gives no start", null, gimbalAim(JSONObject("""{"gimbals":[{"active":true,"pitch":null,"bodyYaw":null}]}""")))
        assertEquals(null, gimbalAim(null))
    }

    @Test
    fun `a second drag carries on from the first instead of snapping back to a stale report`() {
        val reported = -2.0 to 0.0
        val sent = -20.0 to 75.0
        assertEquals("the gimbal is still on its way to the last aim, so that is where the finger left it", sent, dragStart(reported, sent, sentAgoMs = 300))
        assertEquals("long after, the gimbal's own report wins, in case something else moved it", reported, dragStart(reported, sent, sentAgoMs = 5000))
        assertEquals(0.0 to 0.0, dragStart(null, null, sentAgoMs = 0))
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

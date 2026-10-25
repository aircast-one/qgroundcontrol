package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CameraSwitchTest {
    private fun camera(slot: Int, active: Boolean = false, problem: String? = null) =
        """{"slot":$slot,"title":"Camera ${slot + 1}","short":"Cam ${slot + 1}","active":$active,"status":"live","problem":${problem?.let { "\"$it\"" } ?: "null"}}"""

    private fun reading(vararg cameras: String, pip: String = """{"enabled":false,"slot":null}""") =
        camerasReading(JSONObject("""{"class":"Cameras","pip":$pip,"cameras":[${cameras.joinToString(",")}]}"""))

    @Test
    fun `the picker hides until there are two cameras to choose between`() {
        assertNull(cameraSwitchState(null))
        assertNull(cameraSwitchState(reading(camera(0, active = true))))
        assertNull(cameraSwitchState(reading(camera(0, active = true), camera(1, problem = "No address"))))
    }

    @Test
    fun `the picker lists every camera that can play, the one on screen chosen`() {
        val state = cameraSwitchState(reading(camera(0, active = true, problem = "No address"), camera(1), camera(2, problem = "No address"), camera(3)))!!
        assertEquals(listOf(0, 1, 3), state.cameras.map { it.slot })
        assertEquals(0, state.shown.slot)
        assertEquals("with no camera on screen the first one is chosen", 0, cameraSwitchState(reading(camera(0), camera(1)))!!.shown.slot)
    }

    @Test
    fun `the second camera shows only when the split is switched on and there is one`() {
        val cameras = arrayOf(camera(0, active = true), camera(1))
        assertEquals(1, pipCamera(reading(*cameras, pip = """{"enabled":true,"slot":1}"""), streamOn = true)?.slot)
        assertNull(pipCamera(reading(*cameras, pip = """{"enabled":false,"slot":1}"""), streamOn = true))
        assertNull(pipCamera(reading(*cameras, pip = """{"enabled":true,"slot":null}"""), streamOn = true))
    }

    @Test
    fun `the split button flips the setting, and hides with no second camera`() {
        val cameras = arrayOf(camera(0, active = true), camera(1))
        assertEquals(true, splitToggleTarget(reading(*cameras, pip = """{"enabled":false,"slot":1}"""), room = true, streamOn = true))
        assertEquals(false, splitToggleTarget(reading(*cameras, pip = """{"enabled":true,"slot":1}"""), room = true, streamOn = true))
        assertNull(splitToggleTarget(reading(*cameras, pip = """{"enabled":true,"slot":null}"""), room = true, streamOn = true))
        assertNull(splitToggleTarget(null, room = true, streamOn = true))
        assertNull("the split needs the video on screen", splitToggleTarget(reading(*cameras, pip = """{"enabled":false,"slot":1}"""), room = false, streamOn = true))
    }

    @Test
    fun `with video switched off neither the second camera nor the split button shows, even before the core catches up`() {
        val stale = reading(camera(0, active = true), camera(1), pip = """{"enabled":true,"slot":1}""")
        assertNull(pipCamera(stale, streamOn = false))
        assertNull(splitToggleTarget(stale, room = true, streamOn = false))
    }

    @Test
    fun `a swipe steps to the neighbouring camera and wraps around the ends`() {
        val state = cameraSwitchState(reading(camera(0), camera(1, active = true), camera(2)))
        assertEquals(2, neighbourCamera(state, 1)?.slot)
        assertEquals(0, neighbourCamera(state, -1)?.slot)
        assertEquals(0, neighbourCamera(cameraSwitchState(reading(camera(0), camera(1), camera(2, active = true))), 1)?.slot)
        assertNull("one camera has nowhere to swipe to", neighbourCamera(cameraSwitchState(reading(camera(0, active = true))), 1))
    }

    @Test
    fun `the chip names the camera on screen, or the first one when none is`() {
        assertEquals(1, chipCamera(reading(camera(0), camera(1, active = true)))?.slot)
        assertEquals(0, chipCamera(reading(camera(0), camera(1)))?.slot)
        assertNull(chipCamera(reading()))
        assertNull(chipCamera(null))
    }

    @Test
    fun `the pilot's pick shows at once and holds until the core sends a camera list made after it`() {
        val shown = JSONObject("""{"class":"Cameras","cameras":[${camera(0, active = true)},${camera(1)}]}""")
        assertEquals("the pick moves the active camera before the core answers", listOf(false, true), pickedReading(shown, CameraPick(1, shown))?.cameras?.map { it.active })
        assertEquals(listOf(true, false), pickedReading(shown, null)?.cameras?.map { it.active })
        val answered = JSONObject("""{"class":"Cameras","cameras":[${camera(0, active = true)},${camera(1)}]}""")
        assertEquals("a list the core sent after the pick is the core's word, even when something else moved the camera back", listOf(true, false), pickedReading(answered, CameraPick(1, shown))?.cameras?.map { it.active })
        assertNull(pickedReading(null, CameraPick(1, null)))
    }

    @Test
    fun `the synthetic camera counts as chosen only when it is the one on screen`() {
        val synthetic = """{"slot":1,"title":"Synthetic","short":"Synthetic","source":"$SYNTHETIC_SOURCE","active":true,"status":"idle","problem":null}"""
        assertEquals(true, syntheticChosen(reading(camera(0), synthetic)))
        assertEquals(false, syntheticChosen(reading(camera(0, active = true), synthetic.replace("\"active\":true", "\"active\":false"))))
        assertEquals(false, syntheticChosen(null))
    }
}

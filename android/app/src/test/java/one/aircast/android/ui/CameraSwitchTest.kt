package one.aircast.android.ui

import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.IntSize
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class CameraSwitchTest {
    private fun camera(slot: Int, active: Boolean = false, problem: String? = null) =
        """{"slot":$slot,"title":"Camera ${slot + 1}","short":"Cam ${slot + 1}","active":$active,"status":"live","problem":${problem?.let { "\"$it\"" } ?: "null"}}"""

    private fun reading(vararg cameras: String, pip: String = """{"enabled":false,"slot":null}""") =
        camerasReading(JSONObject("""{"class":"Cameras","pip":$pip,"cameras":[${cameras.joinToString(",")}]}"""))

    @Test
    fun `the switch hides until there are two cameras to switch between`() {
        assertNull(cameraSwitchState(null))
        assertNull(cameraSwitchState(reading(camera(0, active = true))))
        assertNull(cameraSwitchState(reading(camera(0, active = true), camera(1, problem = "No address"))))
    }

    @Test
    fun `with two cameras a tap shows the other one`() {
        val state = cameraSwitchState(reading(camera(0), camera(1, active = true)))!!
        assertEquals(1, state.shown.slot)
        assertEquals(0, state.toggleTo)
    }

    @Test
    fun `with more cameras a tap opens the list, which leaves out the ones that cannot play`() {
        val state = cameraSwitchState(reading(camera(0, active = true, problem = "No address"), camera(1), camera(2, problem = "No address"), camera(3)))!!
        assertNull(state.toggleTo)
        assertEquals(listOf(0, 1, 3), state.cameras.map { it.slot })
        assertEquals(0, state.shown.slot)
    }

    @Test
    fun `the second camera shows only when it is switched on and there is one`() {
        val cameras = arrayOf(camera(0, active = true), camera(1))
        assertEquals(1, pipCamera(reading(*cameras, pip = """{"enabled":true,"slot":1}"""), streamOn = true)?.slot)
        assertNull(pipCamera(reading(*cameras, pip = """{"enabled":false,"slot":1}"""), streamOn = true))
        assertNull(pipCamera(reading(*cameras, pip = """{"enabled":true,"slot":null}"""), streamOn = true))
    }

    @Test
    fun `the picture-in-picture button flips the setting, and hides with no second camera`() {
        val cameras = arrayOf(camera(0, active = true), camera(1))
        assertEquals(true, pipToggleTarget(reading(*cameras, pip = """{"enabled":false,"slot":1}"""), thumbnailRoom = true, streamOn = true))
        assertEquals(false, pipToggleTarget(reading(*cameras, pip = """{"enabled":true,"slot":1}"""), thumbnailRoom = true, streamOn = true))
        assertNull(pipToggleTarget(reading(*cameras, pip = """{"enabled":true,"slot":null}"""), thumbnailRoom = true, streamOn = true))
        assertNull(pipToggleTarget(null, thumbnailRoom = true, streamOn = true))
    }

    @Test
    fun `the picture-in-picture button hides where the view has no room for the thumbnail`() {
        assertNull(pipToggleTarget(reading(camera(0, active = true), camera(1), pip = """{"enabled":false,"slot":1}"""), thumbnailRoom = false, streamOn = true))
    }

    @Test
    fun `with video switched off neither the second camera nor its button shows, even before the core catches up`() {
        val stale = reading(camera(0, active = true), camera(1), pip = """{"enabled":true,"slot":1}""")
        assertNull(pipCamera(stale, streamOn = false))
        assertNull(pipToggleTarget(stale, thumbnailRoom = true, streamOn = false))
        val off = reading(camera(0, active = true), camera(1), pip = """{"enabled":false,"slot":1}""")
        assertNull("the button never offers a switch-on that cannot show anything", pipToggleTarget(off, thumbnailRoom = true, streamOn = false))
    }

    @Test
    fun `a double tap on the two-camera switch sends one switch`() {
        val first = SwitchTap(from = 0, atMs = 1_000)
        assertTrue(switchTapAllowed(null, shown = 0, nowMs = 1_000))
        assertFalse("the second tap of a double tap, before the core answers", switchTapAllowed(first, shown = 0, nowMs = 1_150))
        assertFalse("the second tap of a double tap, after the core already switched", switchTapAllowed(first, shown = 1, nowMs = 1_200))
        assertFalse("a slow repeat while the switch is still pending", switchTapAllowed(first, shown = 0, nowMs = 1_900))
        assertTrue("once the camera changed, the next tap switches back", switchTapAllowed(first, shown = 1, nowMs = 1_400))
        assertTrue("a switch the core never answered stops blocking", switchTapAllowed(first, shown = 0, nowMs = 3_000))
    }

    @Test
    fun `with no camera on screen the switch shows the first one`() {
        val state = cameraSwitchState(reading(camera(0), camera(1)))!!
        assertEquals(0, state.shown.slot)
        assertEquals(1, state.toggleTo)
    }

    private val window = IntSize(2400, 1080)
    private val menu = IntSize(400, 300)

    private fun IntOffset.covers(rect: IntRect) = IntRect(this, menu).overlaps(rect)

    @Test
    fun `in landscape the list opens toward the picture, clear of the pill and the shutter rail beside it`() {
        val pill = IntRect(2000, 500, 2120, 560)
        val shutter = IntRect(2136, 440, 2380, 640)
        val at = menuBeside(pill, window, menu, MenuSide.Start, gap = 16, rtl = false)
        assertEquals(IntOffset(1584, 380), at)
        assertEquals(false, at.covers(pill))
        assertEquals(false, at.covers(shutter))
    }

    @Test
    fun `in portrait the list opens above the pill, its edge in line with the pill's`() {
        val portrait = IntSize(1080, 2400)
        val pill = IntRect(700, 2000, 820, 2060)
        val at = menuBeside(pill, portrait, menu, MenuSide.Above, gap = 16, rtl = false)
        assertEquals(IntOffset(420, 1684), at)
        assertEquals(false, at.covers(pill))
    }

    @Test
    fun `the list stays on screen when the pill sits near an edge`() {
        assertEquals(IntOffset(1584, 0), menuBeside(IntRect(2000, 0, 2120, 60), window, menu, MenuSide.Start, gap = 16, rtl = false))
        assertEquals(IntOffset(0, 0), menuBeside(IntRect(100, 100, 220, 160), IntSize(1080, 2400), menu, MenuSide.Above, gap = 16, rtl = false))
    }

    @Test
    fun `right to left the rail is on the left, so the list opens to the pill's right`() {
        val pill = IntRect(280, 500, 400, 560)
        assertEquals(IntOffset(416, 380), menuBeside(pill, window, menu, MenuSide.Start, gap = 16, rtl = true))
    }
}

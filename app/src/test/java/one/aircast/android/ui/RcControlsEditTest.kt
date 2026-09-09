package one.aircast.android.ui

import org.json.JSONArray
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class RcControlsEditTest {

    private val served = """
        [{"label":"Winch","channel":7,"type":"slider","orientation":"vertical"},
         {"label":"Light","channel":9,"type":"button"}]
    """

    @Test
    fun `editing a control keeps the fields this head does not understand`() {
        val next = rcControlsPatched(served, 0, "Hoist", 8, RcControlType.Button)
        val entry = JSONArray(next).getJSONObject(0)

        assertEquals("Hoist", entry.getString("label"))
        assertEquals(8, entry.getInt("channel"))
        assertEquals("button", entry.getString("type"))
        assertEquals("vertical", entry.getString("orientation"))
    }

    @Test
    fun `adding and removing keep the rest of the list intact`() {
        val added = rcControlsAdded(served, "Zoom", 11, RcControlType.Switch3)
        assertEquals(listOf("Winch", "Light", "Zoom"), parseRcControls(added).map { it.label })
        assertEquals(RcControlType.Switch3, parseRcControls(added)[2].type)

        val removed = rcControlsRemoved(added, 1)
        assertEquals(listOf("Winch", "Zoom"), parseRcControls(removed).map { it.label })
        assertEquals("vertical", JSONArray(removed).getJSONObject(0).getString("orientation"))
    }

    @Test
    fun `a channel already driving something names what has it`() {
        val reserved = mapOf(5 to "Gimbal tilt")

        assertEquals("Gimbal tilt", channelOwner(served, 5, -1, reserved))
        assertEquals("Light", channelOwner(served, 9, 0, reserved))
        assertNull(channelOwner(served, 9, 1, reserved))
        assertNull(channelOwner(served, 12, -1, reserved))
    }

    @Test
    fun `an unnamed control still identifies itself in a clash`() {
        val unnamed = """[{"label":"","channel":4,"type":"button"}]"""

        assertEquals("control 1", channelOwner(unnamed, 4, -1, emptyMap()))
    }

    @Test
    fun `a new control lands on the first channel nothing else is using`() {
        assertEquals(1, firstFreeChannel(served, emptyMap()))
        assertEquals(2, firstFreeChannel(served, mapOf(1 to "Gimbal tilt")))
        assertEquals(8, firstFreeChannel("""[{"channel":1,"type":"button"},{"channel":2,"type":"button"}]""",
            mapOf(3 to "a", 4 to "b", 5 to "c", 6 to "d", 7 to "e")))
    }

    @Test
    fun `channels outside the radio's range are refused`() {
        assertEquals(false, channelUsable(0))
        assertEquals(false, channelUsable(19))
        assertEquals(true, channelUsable(1))
        assertEquals(true, channelUsable(18))
    }

    @Test
    fun `a type round trips through its stored name`() {
        RcControlType.entries.forEach { type ->
            val json = rcControlsAdded(null, "x", 3, type)
            assertEquals(type, parseRcControls(json).single().type)
        }
    }
}

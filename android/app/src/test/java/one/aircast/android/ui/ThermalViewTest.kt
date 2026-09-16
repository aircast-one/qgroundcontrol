package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ThermalViewTest {

    private fun camera(
        available: Boolean = true,
        mode: String = "\"blend\"",
        opacity: String = "60.0",
    ) = JSONObject(
        """{"kind":"object","class":"Camera","present":true,"thermalAvailable":$available,
            "thermalMode":$mode,"thermalOpacity":$opacity}""",
    )

    @Test
    fun `a camera with no thermal stream offers no thermal controls`() {
        assertNull(
            "PhotoVideoControl gates the whole control on there being a thermal stream, so a " +
                "plain camera has no mode rather than a mode of off",
            thermalReading(camera(available = false, mode = "null", opacity = "null")),
        )
        assertNull(thermalReading(null))
    }

    @Test
    fun `the served token is the mode, not an index into a list`() {
        assertEquals("blend", thermalReading(camera())!!.mode)
        assertEquals("picInPic", thermalReading(camera(mode = "\"picInPic\""))!!.mode)
    }

    @Test
    fun `every served token has a word, and an unknown one is shown rather than hidden`() {
        assertEquals("Off", thermalModeLabel("off"))
        assertEquals("Blend", thermalModeLabel("blend"))
        assertEquals("Full", thermalModeLabel("full"))
        assertEquals("Picture in picture", thermalModeLabel("picInPic"))
        assertEquals(
            "a token the core adds later is drawn as itself; dropping it would leave the " +
                "operator a mode they cannot see they are in",
            "somethingNew",
            thermalModeLabel("somethingNew"),
        )
    }

    @Test
    fun `opacity is offered only when the core sends one`() {
        assertTrue(
            "the core sends opacity only while blending, so its presence is the gate - a null " +
                "means do not draw the slider, not draw it at zero",
            thermalOpacityIsOffered(thermalReading(camera())),
        )
        assertFalse(thermalOpacityIsOffered(thermalReading(camera(mode = "\"full\"", opacity = "null"))))
        assertFalse(thermalOpacityIsOffered(null))
    }

    @Test
    fun `a null opacity is absent, not zero`() {
        val full = thermalReading(camera(mode = "\"full\"", opacity = "null"))!!

        assertEquals("full", full.mode)
        assertNull(
            "optDouble would have flattened a null to NaN and a bad read to 0.0; a slider at " +
                "zero claims the blend is fully transparent, which is a different statement",
            full.opacity,
        )
    }

    @Test
    fun `the write order matches the tokens the core serves`() {
        assertEquals(listOf("off", "blend", "full", "picInPic"), THERMAL_MODES)
        assertEquals(
            "the property is a Q_ENUM written by index, so this list is the enum's order and " +
                "not a display preference - reordering it would set the wrong mode",
            1,
            THERMAL_MODES.indexOf("blend"),
        )
    }
}

package one.aircast.android.ui

import one.aircast.android.bridge.Fact
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SlideToConfirmTest {
    @Test
    fun `a tap does not confirm`() {
        assertFalse(slideConfirms(slideFraction(offsetPx = 0f, trackPx = 1000, thumbPx = 150f)))
    }

    @Test
    fun `a short nudge does not confirm`() {
        assertFalse(slideConfirms(slideFraction(offsetPx = 200f, trackPx = 1000, thumbPx = 150f)))
    }

    @Test
    fun `most of the way across still does not confirm`() {
        val fraction = slideFraction(offsetPx = 700f, trackPx = 1000, thumbPx = 150f)
        assertTrue(fraction > 0.8f)
        assertFalse(slideConfirms(fraction))
    }

    @Test
    fun `sliding to the end confirms`() {
        assertTrue(slideConfirms(slideFraction(offsetPx = 850f, trackPx = 1000, thumbPx = 150f)))
    }

    @Test
    fun `the fraction never leaves zero to one`() {
        assertEquals(0f, slideFraction(-500f, 1000, 150f), 0.0001f)
        assertEquals(1f, slideFraction(99999f, 1000, 150f), 0.0001f)
    }

    @Test
    fun `a track narrower than its thumb cannot confirm`() {
        assertEquals(0f, slideFraction(50f, 100, 150f), 0.0001f)
        assertFalse(slideConfirms(slideFraction(50f, 100, 150f)))
    }

    @Test
    fun `an unmeasured track cannot confirm`() {
        assertFalse(slideConfirms(slideFraction(0f, 0, 150f)))
    }
}

class TelemetryFormatTest {
    private fun fact(name: String, description: String, units: String, value: String) = Fact(
        path = "vehicle.$name",
        name = name,
        description = description,
        units = units,
        valueString = value,
        value = value,
        enumStrings = emptyList(),
        enumIndex = 0,
        isBool = false,
        isString = false,
        readOnly = true,
    )

    @Test
    fun `a pilot sees the described name, not the property name`() {
        assertEquals(
            "Altitude Rel",
            telemetryLabel(fact("altitudeRelative", "Altitude Rel", "m", "12.0")),
        )
    }

    @Test
    fun `a property with no description falls back to its name`() {
        assertEquals("heading", telemetryLabel(fact("heading", "", "deg", "356")))
    }

    @Test
    fun `a value carries its units`() {
        assertEquals("12.0 m", telemetryValue(fact("altitudeRelative", "Altitude Rel", "m", "12.0")))
    }

    @Test
    fun `a unitless value is shown bare`() {
        assertEquals("356", telemetryValue(fact("heading", "Heading", "", "356")))
    }
}

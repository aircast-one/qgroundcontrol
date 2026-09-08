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

class GuidedAvailabilityTest {
    private fun availability(
        armed: Boolean = true,
        flying: Boolean = true,
        guided: Boolean = true,
        takeoff: Boolean = true,
        fixedWing: Boolean = false,
        mode: String = "Guided",
        landMode: String = "Land",
        rtlMode: String = "RTL",
    ) = guidedAvailability(armed, flying, guided, takeoff, fixedWing, mode, landMode, rtlMode)

    @Test
    fun `the RTL button reads the vehicle's own name for the mode`() {
        val px4 = availability(armed = true, flying = true, guided = true, mode = "Return", rtlMode = "Return")
        val apm = availability(armed = true, flying = true, guided = true, mode = "RTL", rtlMode = "RTL")

        assertFalse(px4.rtl)
        assertFalse(apm.rtl)
    }

    @Test
    fun `takeoff is offered only when the vehicle is on the ground`() {
        assertTrue(availability(flying = false).takeoff)
        assertFalse(availability(flying = true).takeoff)
    }

    @Test
    fun `takeoff is not offered by a vehicle that cannot take off`() {
        assertFalse(availability(flying = false, takeoff = false).takeoff)
    }

    @Test
    fun `land needs an armed vehicle that is not a fixed wing`() {
        assertTrue(availability().land)
        assertFalse(availability(armed = false).land)
        assertFalse(availability(fixedWing = true).land)
        assertFalse(availability(guided = false).land)
    }

    @Test
    fun `land is not offered while already landing`() {
        assertFalse(availability(mode = "Land").land)
        assertFalse(availability(mode = "land").land)
    }

    @Test
    fun `return needs an armed vehicle that is flying`() {
        assertTrue(availability().rtl)
        assertFalse(availability(flying = false).rtl)
        assertFalse(availability(armed = false).rtl)
    }

    @Test
    fun `return is not offered while already returning`() {
        assertFalse(availability(mode = "RTL").rtl)
        assertFalse(availability(mode = "rtl").rtl)
    }

    @Test
    fun `a disarmed vehicle on the ground offers only takeoff`() {
        val can = availability(armed = false, flying = false)
        assertTrue(can.takeoff)
        assertFalse(can.land)
        assertFalse(can.rtl)
    }
}

class VehicleSubtitleTest {
    @Test
    fun `a lost link is named instead of the last known state`() {
        assertEquals(
            "Communication lost",
            vehicleSubtitle(available = true, communicationLost = true, flightMode = "Stabilize", armed = false),
        )
    }

    @Test
    fun `a live vehicle reads mode and armed state`() {
        assertEquals(
            "Stabilize · Disarmed",
            vehicleSubtitle(available = true, communicationLost = false, flightMode = "Stabilize", armed = false),
        )
        assertEquals(
            "Guided · Armed",
            vehicleSubtitle(available = true, communicationLost = false, flightMode = "Guided", armed = true),
        )
    }

    @Test
    fun `no vehicle outranks a lost link`() {
        assertEquals(
            "No vehicle",
            vehicleSubtitle(available = false, communicationLost = true, flightMode = "Stabilize", armed = true),
        )
    }

    @Test
    fun `a missing flight mode does not leave a dangling separator`() {
        assertEquals(
            "Armed",
            vehicleSubtitle(available = true, communicationLost = false, flightMode = "", armed = true),
        )
    }
}

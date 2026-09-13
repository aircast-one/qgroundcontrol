package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

private fun component(name: String, known: String? = null) =
    SetupComponent(index = 0, name = name, known = known, needsAttention = false)

class SetupPagesTest {
    @Test
    fun `a component with a stable identity is routed by it, not by its display name`() {
        assertEquals(SENSORS, headPage(component("Sensoren", known = "sensors")))
        assertEquals(RADIO, headPage(component("Funk", known = "radio")))
        assertEquals(FLIGHT_MODES_PAGE, headPage(component("Flugmodi", known = "flightModes")))
    }

    @Test
    fun `a component QGC has no identity for falls back to its name, which is all there is`() {
        assertEquals(MOTORS, headPage(component(MOTORS)))
        assertEquals(REMOTE_SUPPORT, headPage(component(REMOTE_SUPPORT)))
    }

    @Test
    fun `an identity the head has no page for keeps the name rather than resolving to nothing`() {
        assertEquals("Power", headPage(component("Power", known = "power")))
        assertEquals("Safety", headPage(component("Safety", known = "safety")))
    }
}

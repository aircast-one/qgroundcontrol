package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class OrbitTest {

    private fun view(
        orbiting: String = "true",
        radiusText: String = "\"120 m\"",
        clockwise: String = "true",
        reason: String = "\"\"",
    ) = JSONObject(
        """{"kind":"object","class":"Orbit","available":true,"orbiting":$orbiting,
           "radiusText":$radiusText,"clockwise":$clockwise,"reason":$reason}""",
    )

    @Test
    fun `another view is not an orbit reading`() {
        assertNull(orbitReading(null))
        assertNull(orbitReading(JSONObject("""{"kind":"object","class":"Orbit2"}""")))
    }

    @Test
    fun `an orbit states its radius and which way round it goes`() {
        assertEquals("Orbiting 120 m clockwise", orbitLabel(orbitReading(view())))
        assertEquals(
            "Orbiting 120 m anticlockwise",
            orbitLabel(orbitReading(view(clockwise = "false"))),
        )
    }

    @Test
    fun `a vehicle not orbiting shows nothing, because the chip is the state`() {
        assertNull(orbitLabel(orbitReading(view(orbiting = "false", radiusText = "null", clockwise = "null"))))
    }

    @Test
    fun `no contact is not a stopped orbit, and neither draws the chip`() {
        val quiet = orbitReading(
            view(
                orbiting = "null",
                radiusText = "null",
                clockwise = "null",
                reason = "\"No contact, so whether the vehicle is still orbiting is unknown.\"",
            ),
        )!!

        assertNull(quiet.orbiting)
        assertNull(orbitLabel(quiet))
        assertEquals("No contact, so whether the vehicle is still orbiting is unknown.", quiet.reason)
    }

    @Test
    fun `an orbit whose radius the core will not state is still an orbit`() {
        assertEquals("Orbiting clockwise", orbitLabel(orbitReading(view(radiusText = "null"))))
    }
}

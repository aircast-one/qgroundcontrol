package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Test

class ComplexKeyTest {
    private fun plan(vararg elements: String) =
        JSONObject("""{"kind":"object","elements":[${elements.joinToString(",")}]}""")

    private fun survey(latitude: Double, longitude: Double, distance: Double) =
        """{"coordinate":{"latitude":$latitude,"longitude":$longitude},"complexDistance":$distance}"""

    @Test
    fun `two plans of the same length over different ground do not share a key`() {
        assertNotEquals(
            complexKey(plan("{}", survey(41.7, 44.8, 500.0)), 1),
            complexKey(plan("{}", survey(47.4, 8.5, 500.0)), 1),
        )
    }

    @Test
    fun `a survey that grew is not the survey that was read`() {
        assertNotEquals(
            complexKey(plan("{}", survey(41.7, 44.8, 500.0)), 1),
            complexKey(plan("{}", survey(41.7, 44.8, 900.0)), 1),
        )
    }

    @Test
    fun `the same survey read twice keys the same`() {
        assertEquals(
            complexKey(plan("{}", survey(41.7, 44.8, 500.0)), 1),
            complexKey(plan("{}", survey(41.7, 44.8, 500.0)), 1),
        )
    }

    @Test
    fun `an index past the end has no key rather than throwing`() {
        assertEquals("", complexKey(plan("{}"), 4))
    }

    @Test
    fun `an unreadable plan has no key`() {
        assertEquals("", complexKey(null, 1))
    }
}

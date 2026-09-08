package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class ComplexIndicesTest {
    private fun plan(vararg elements: String) =
        JSONObject("""{"kind":"object","elements":[${elements.joinToString(",")}]}""")

    private val simple = """{"specifiesCoordinate":true}"""
    private fun complex(distance: Double) =
        """{"specifiesCoordinate":true,"complexDistance":$distance}"""

    @Test
    fun `only items that cover a distance of their own are complex`() {
        assertEquals(
            listOf(2, 4),
            complexIndices(plan(simple, simple, complex(500.0), simple, complex(10.0))),
        )
    }

    @Test
    fun `a complex item with no distance yet is not one to read segments for`() {
        assertEquals(emptyList<Int>(), complexIndices(plan(simple, complex(0.0), simple)))
    }

    @Test
    fun `the settings item is never one`() {
        assertEquals(emptyList<Int>(), complexIndices(plan(complex(500.0))))
    }

    @Test
    fun `an unreadable plan has none`() {
        assertEquals(emptyList<Int>(), complexIndices(null))
    }
}

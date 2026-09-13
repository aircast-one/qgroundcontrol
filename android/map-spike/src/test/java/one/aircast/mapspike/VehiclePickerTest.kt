package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class VehiclePickerTest {
    private fun list(vararg ids: Int, activeId: Int = -1) =
        JSONObject(
            """{"kind":"object","vehicles":[${
                ids.joinToString(",") { """{"id":$it,"active":${it == activeId}}""" }
            }]}""",
        )

    @Test
    fun `each connected vehicle is an entry named by its id`() {
        val entries = vehicleEntries(list(1, 2, activeId = 2))

        assertEquals(
            "the id is what selects an aircraft; a list position can shift between read and write",
            listOf(1, 2),
            entries.map { it.id },
        )
    }

    @Test
    fun `the active one is marked and the others are not`() {
        val entries = vehicleEntries(list(1, 2, activeId = 2))

        assertEquals(listOf(false, true), entries.map { it.active })
    }

    @Test
    fun `the head does not decide which is active, the core does`() {
        assertTrue(vehicleEntries(list(1, 2)).none { it.active })
    }

    @Test
    fun `an entry without an id is not offered`() {
        val json = JSONObject("""{"kind":"object","vehicles":[{"id":1},{}]}""")

        assertEquals(1, vehicleEntries(json).size)
    }

    @Test
    fun `nothing connected is an empty list rather than a failure`() {
        assertTrue(vehicleEntries(null).isEmpty())
    }

}

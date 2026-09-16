package one.aircast.android.ui

import org.json.JSONArray
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class UndrawnItemsTest {

    private fun items(vararg pairs: Pair<String, String>) = JSONArray(
        "[" + pairs.joinToString(",") { (kind, name) ->
            """{"kind":"$kind","name":"$name"}"""
        } + "]",
    )

    @Test
    fun `the two kinds the map cannot draw are the two it warns about`() {
        assertEquals(
            "missionitems.rs kind() returns complex when by_class does not recognise the item's " +
                "class, and unreadable when QGC calls it neither simple nor complex - those are " +
                "the only two outside DRAWN_KINDS, and they are exactly what has no shape to draw",
            listOf("Orbit", "Unknown: 1234"),
            undrawnItemNames(items("complex" to "Orbit", "unreadable" to "Unknown: 1234")),
        )
    }

    @Test
    fun `Mission Start is drawn, so no plan is warned about its own first row`() {
        assertEquals(
            "every plan carries a settings item - live capture shows kind settings, name Mission " +
                "Start - so leaving it out of DRAWN_KINDS would put a warning on every plan ever " +
                "opened",
            emptyList<String>(),
            undrawnItemNames(items("settings" to "Mission Start", "takeoff" to "Takeoff")),
        )
    }

    @Test
    fun `every drawable kind stays out of the warning`() {
        DRAWN_KINDS.forEach { kind ->
            assertEquals(kind, emptyList<String>(), undrawnItemNames(items(kind to "Something")))
        }
    }

    @Test
    fun `one name is not repeated when several items share it`() {
        assertEquals(
            listOf("Orbit"),
            undrawnItemNames(items("complex" to "Orbit", "complex" to "Orbit")),
        )
    }

    @Test
    fun `an undrawn item with no name is left out of the sentence entirely`() {
        assertEquals(
            "deliberate: an out-of-date AAR yields items with no class and so no name, and naming " +
                "nothing is better than naming it wrongly. The cost is that a plan whose undrawn " +
                "items are ALL unnamed gets no warning at all, which is recorded rather than fixed",
            emptyList<String>(),
            undrawnItemNames(items("complex" to "")),
        )
        assertNull(undrawnItemsWarning(undrawnItemNames(items("complex" to ""))))
    }

    @Test
    fun `the sentence says the items are still flown, not merely undrawn`() {
        assertEquals(
            "a pilot who reads only 'the map cannot draw X' may take it as cosmetic; the item " +
                "still counts in distance and duration and still uploads",
            "The map cannot draw Orbit. Those items are still in the plan and will still be flown.",
            undrawnItemsWarning(listOf("Orbit")),
        )
        assertNull(undrawnItemsWarning(emptyList()))
    }
}

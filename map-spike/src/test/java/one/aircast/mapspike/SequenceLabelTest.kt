package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class SequenceLabelTest {

    private fun item(sequence: Int, folded: Int) = MissionItem(
        sequence, sequence, 41.0, 44.0, "Waypoint", false, 50.0,
        kind = "waypoint", foldedCommands = folded,
    )

    @Test
    fun `an item that folds nothing is numbered plainly`() {
        assertEquals("2", sequenceLabel(item(2, 0)))
    }

    @Test
    fun `an item that folds a command owns the span, so the next number is not a hole`() {
        assertEquals("2–3", sequenceLabel(item(2, 1)))
    }

    @Test
    fun `a survey owns every sequence it generates`() {
        assertEquals("4–216", sequenceLabel(item(4, 212)))
    }

    @Test
    fun `the rows of a plan with a folded speed leave no gap between them`() {
        val rows = itemRows(listOf(item(0, 0), item(1, 0), item(2, 1), item(4, 0)))

        assertEquals(listOf("0", "1", "2–3", "4"), rows.map { it.number })
    }

    @Test
    fun `an item the core said nothing about is numbered plainly rather than given a range`() {
        assertEquals("2", sequenceLabel(item(2, 0)))
        assertEquals("2", sequenceLabel(item(2, -1)))
    }
}

package one.aircast.map

import org.junit.Assert.assertEquals
import org.junit.Test

class SettingStepperTest {
    @Test
    fun `a step moves by its size and stops at the ends of the range`() {
        assertEquals(listOf(51.0, 49.0, 500.0, 0.0, -3.0), listOf(
            steppedValue(50.0, 1.0, 1, 0.0..500.0),
            steppedValue(50.0, 1.0, -1, 0.0..500.0),
            steppedValue(500.0, 1.0, 1, 0.0..500.0),
            steppedValue(0.0, 1.0, -1, 0.0..500.0),
            steppedValue(-2.0, 1.0, -1, null),
        ))
    }

    @Test
    fun `whole numbers read whole and a half step shows its decimal`() {
        assertEquals(listOf(0, 1, 1), listOf(stepperDecimals(50.0, 1.0), stepperDecimals(5.0, 0.5), stepperDecimals(45.5, 1.0)))
        assertEquals(listOf("50", "5.5", "—"), listOf(stepperText(50.0, 0), stepperText(5.5, 1), stepperText(null, 0)))
    }

    @Test
    fun `the altitude slider spans the same height in either unit`() {
        assertEquals(500.0, altitudeRange("m").endInclusive, 0.0)
        assertEquals(1640.0, altitudeRange("ft").endInclusive, 0.0)
    }

    @Test
    fun `the waypoint strip leaves out the planned home, which is not flown to`() {
        val rows = listOf(0, 1, 2).map { ItemRow(it, "$it", "Waypoint", "", "#ffffff", true) }
        assertEquals(listOf(1, 2), stripRows(rows).map { it.index })
    }
}

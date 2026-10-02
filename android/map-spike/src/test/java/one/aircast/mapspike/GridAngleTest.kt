package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class GridAngleTest {
    @Test
    fun `the grid angle reads as a whole degree from 0 to 359, like SurveyItemEditor's slider`() {
        assertEquals(0f, gridAngleShown(Double.NaN), 0f)
        assertEquals(45f, gridAngleShown(44.6), 0f)
        assertEquals(350f, gridAngleShown(-10.0), 0f)
        assertEquals(0f, gridAngleShown(360.0), 0f)
        assertEquals(359f, gridAngleShown(359.2), 0f)
    }
}

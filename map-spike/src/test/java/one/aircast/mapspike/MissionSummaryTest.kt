package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class MissionSummaryTest {
    @Test
    fun `short plans stay in metres and long ones turn into kilometres`() {
        assertEquals("450 m", missionSummary(450.0, Double.NaN))
        assertEquals("1.50 km", missionSummary(1500.0, Double.NaN))
    }

    @Test
    fun `time reads as minutes and seconds until it needs hours`() {
        assertEquals("2:05", missionSummary(Double.NaN, 125.0))
        assertEquals("1:00:30", missionSummary(Double.NaN, 3630.0))
    }

    @Test
    fun `distance and time are joined when both are known`() {
        assertEquals("2.00 km · 4:00", missionSummary(2000.0, 240.0))
    }

    @Test
    fun `an uncosted or empty plan summarises to nothing`() {
        assertEquals("", missionSummary(Double.NaN, Double.NaN))
        assertEquals("", missionSummary(0.0, 0.0))
    }
}

package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class VehicleStatusSheetTest {
    @Test
    fun `the status sheet lists faulty sensors until asked for the rest`() {
        val sensors = listOf(SensorHealth("Gyro", "unhealthy", "Error"), SensorHealth("Baro", "healthy", "Normal"))
        assertEquals(listOf("Gyro"), shownSensors(sensors, showAll = false).map { it.name })
        assertEquals(listOf("Gyro", "Baro"), shownSensors(sensors, showAll = true).map { it.name })
    }
}

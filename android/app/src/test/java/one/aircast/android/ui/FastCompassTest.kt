package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class FastCompassTest {
    private val withGcs = fastCompass(JSONObject("""{"invocation":"sensorsCal.calibrateCompassNorth","help":"h","vehicleHasPosition":false,
        "gcsPosition":{"valid":true,"latitude":41.7,"longitude":44.8}}"""))!!
    private val withoutGcs = fastCompass(JSONObject("""{"invocation":"sensorsCal.calibrateCompassNorth","help":"h","vehicleHasPosition":true,
        "gcsPosition":{"valid":false,"latitude":null,"longitude":null}}"""))!!

    @Test
    fun `a valid gcs position is used unless the operator unticks it, like qgc`() {
        val choice = initialFastCompassChoice(withGcs).copy(enabled = true)
        assertTrue(choice.useGcs)
        assertEquals(listOf(41.7, 44.8), fastCompassArguments(withGcs, choice))
        assertEquals(listOf<Any>(1.5, 2.5), fastCompassArguments(withGcs, choice.copy(useGcs = false, latitude = "1.5", longitude = "2.5")))
    }

    @Test
    fun `without a gcs position the typed coordinates go to the core, which rejects garbage`() {
        val choice = initialFastCompassChoice(withoutGcs).copy(enabled = true)
        assertFalse(choice.useGcs)
        assertNull(withoutGcs.gcsLatitude)
        assertEquals(listOf<Any>(0.0, 0.0), fastCompassArguments(withoutGcs, choice))
        assertEquals(listOf<Any>("x", 0.0), fastCompassArguments(withoutGcs, choice.copy(latitude = "x")))
    }

    @Test
    fun `px4 serves no fast compass`() {
        assertNull(fastCompass(null))
    }
}

package one.aircast.android.ui

import one.aircast.android.bridge.Fact
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

private fun fact(
    units: String,
    enums: List<String> = emptyList(),
    bits: List<String> = emptyList(),
    bool: Boolean = false,
) = Fact(
    path = "p",
    name = "N",
    description = "d",
    units = units,
    valueString = "0.15",
    value = "0.15",
    enumStrings = enums,
    enumIndex = 0,
    bitmaskStrings = bits,
    bitmaskValues = bits.indices.map { 1L shl it },
    isBool = bool,
    isString = false,
    readOnly = false,
)

class FactSubtitleTest {
    @Test
    fun `a number keeps the unit it is measured in`() {
        assertEquals("m", factSubtitle(fact(units = "m")))
    }

    @Test
    fun `a row showing a word drops the unit, because Medium is not measured in seconds`() {
        assertEquals("", factSubtitle(fact(units = "s", enums = listOf("Soft", "Medium"))))
        assertEquals("", factSubtitle(fact(units = "s", bits = listOf("Alt", "Circle"))))
        assertEquals("", factSubtitle(fact(units = "s", bool = true)))
    }
}

class RunningTitleTest {
    @Test
    fun `a named routine reads as itself`() {
        assertEquals("Calibrating Compass", runningTitle("Compass"))
    }

    @Test
    fun `a calibration the screen did not start still says what is happening`() {
        assertEquals("Calibration in progress", runningTitle(""))
        assertEquals("Calibration in progress", runningTitle("  "))
    }
}

class OperatorDistanceTest {
    @Test
    fun `the served text is shown as its own reading`() {
        val view = JSONObject("""{"distanceToVehicleText":"316.8 m"}""")

        assertEquals(listOf(Instrument("From you", "316.8 m")), operatorDistance(view))
    }

    @Test
    fun `nothing is shown when the core withholds the distance`() {
        assertEquals(emptyList<Instrument>(), operatorDistance(JSONObject("""{"distanceToVehicleText":null}""")))
        assertEquals(emptyList<Instrument>(), operatorDistance(JSONObject("{}")))
        assertEquals(emptyList<Instrument>(), operatorDistance(null))
    }
}

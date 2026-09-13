package one.aircast.android.ui

import one.aircast.android.bridge.Fact
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

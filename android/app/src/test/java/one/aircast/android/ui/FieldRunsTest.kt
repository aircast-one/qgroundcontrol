package one.aircast.android.ui

import one.aircast.android.bridge.Fact
import org.junit.Assert.assertEquals
import org.junit.Test

class FieldRunsTest {
    private fun choice(name: String, options: List<String> = listOf("Metric", "Imperial", "SI", "Nautical", "Other"), label: String = name) = Fact(
        path = "settings.$name", name = name, description = "$name long description", units = "", valueString = options.first(),
        value = 0, enumStrings = options, enumIndex = 0, isBool = false, isString = false, readOnly = false, shortLabel = label,
    )

    private fun text(name: String, isString: Boolean = true, units: String = "") = Fact(
        path = "settings.$name", name = name, description = name, units = units, valueString = "", value = "",
        enumStrings = emptyList(), enumIndex = -1, isBool = false, isString = isString, readOnly = false, shortLabel = name,
    )

    @Test
    fun consecutiveShortChoicesShareARow() =
        assertEquals(listOf(listOf("a", "b"), listOf("c", "d"), listOf("e")), fieldRuns(listOf("a", "b", "c", "d", "e").map { choice(it) }).map { run -> run.map { it.name } })

    @Test
    fun anythingElseBreaksThePair() =
        assertEquals(listOf(listOf("a"), listOf("t"), listOf("b"), listOf("long")), fieldRuns(listOf(choice("a"), text("t"), choice("b"), choice("long", listOf("A choice far too long", "x", "y", "z", "w")))).map { run -> run.map { it.name } })

    @Test
    fun aChoiceWithoutAShortLabelStandsAlone() =
        assertEquals(2, fieldRuns(listOf(choice("a", label = ""), choice("b"))).size)

    @Test
    fun shortNumbersPairButWideUnitsDoNot() =
        assertEquals(listOf(listOf("speed", "alt"), listOf("rate")), fieldRuns(listOf(text("speed", false, "m/s"), text("alt", false, "m"), text("rate", false, "degrees/second"))).map { run -> run.map { it.name } })
}

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
    fun aSettingWithNoEffectYetStillDrawsAsAFieldAndPairs() {
        val host = text("host", isString = false).copy(enabled = false, disabledReason = "Has no effect while the ADSB server connection is off.")
        assertEquals(true, showsAsField(host))
        assertEquals(listOf(listOf("host", "port")), fieldRuns(listOf(host, host.copy(name = "port"))).map { run -> run.map { it.name } })
        assertEquals(false, showsAsField(host.copy(readOnly = true)))
    }

    @Test
    fun aNumberFieldDropsThePaddedDecimalsButTextIsLeftAlone() {
        assertEquals("300", fieldText(text("d", isString = false).copy(valueString = "300.000")))
        assertEquals("2.5", fieldText(text("d", isString = false).copy(valueString = "2.50")))
        assertEquals("007.10", fieldText(text("s").copy(valueString = "007.10")))
    }

    @Test
    fun aRebootNoticeSharedByTheBlockIsSaidOnce() {
        val reboot = text("a").copy(vehicleRebootRequired = true)
        assertEquals("Reboot vehicle for changes to take effect.", sharedRebootNote(listOf(reboot, text("plain"), reboot.copy(name = "b"))))
        assertEquals(null, sharedRebootNote(listOf(reboot, text("plain"))))
        assertEquals(null, sharedRebootNote(listOf(reboot, text("c").copy(qgcRebootRequired = true))))
    }

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

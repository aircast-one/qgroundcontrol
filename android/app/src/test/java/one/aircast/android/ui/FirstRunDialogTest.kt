package one.aircast.android.ui

import one.aircast.android.bridge.Fact
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class FirstRunDialogTest {
    @Test
    fun `the prompt shows until the core says it was shown`() {
        val prompt = firstRun(
            JSONObject(
                """{"show":true,"title":"Preferences","vehicleHeading":"Vehicle Preferences","vehicleDescription":"d",""" +
                    """"vehiclePreferences":[{"name":"preferredFirmwareClass","label":"Preferred Firmware","control":"choice","path":"settings.appSettings.preferredFirmwareClass"}],""" +
                    """"unitsHeading":"Measurement Units","unitsDescription":"u"}""",
            ),
        )!!
        assertEquals("Preferences", prompt.title)
        assertEquals("Preferred Firmware", prompt.preferences.single().title)
        assertNull(firstRun(JSONObject("""{"show":false}""")))
    }

    private fun fact(name: String) = Fact(
        path = "settings.unitsSettings.$name",
        name = name,
        description = name,
        units = "",
        valueString = "0",
        value = 0,
        enumStrings = listOf("A", "B"),
        enumIndex = 0,
        isBool = false,
        isString = false,
        readOnly = false,
    )

    private val shown = listOf("horizontalDistanceUnits", "verticalDistanceUnits", "areaUnits", "speedUnits", "temperatureUnits", "weightUnits", "customUnits").map(::fact)

    @Test
    fun `the five unit rows always show, like InitialSetupPrompt's repeater`() {
        assertEquals(listOf("horizontalDistanceUnits", "verticalDistanceUnits", "areaUnits", "speedUnits", "temperatureUnits"), firstRunUnitRows(shown).map { it.name })
    }

    @Test
    fun `imperial writes feet, square feet, feet per second and fahrenheit like changeSystemOfUnits`() {
        val rows = firstRunUnitRows(shown)
        assertEquals(listOf(0, 0, 0, 0, 1), firstRunSystemWrites(false, rows).map { it.second })
        assertEquals(listOf(1, 1, 1, 1, 0), firstRunSystemWrites(true, rows).map { it.second })
        assertEquals("settings.unitsSettings.horizontalDistanceUnits", firstRunSystemWrites(true, rows).first().first)
    }

    @Test
    fun `hidden units are not written`() {
        assertEquals(listOf("settings.unitsSettings.speedUnits"), firstRunSystemWrites(true, firstRunUnitRows(listOf(fact("speedUnits")))).map { it.first })
    }

    @Test
    fun `metric is chosen only while horizontal distance is in meters`() {
        assertEquals(0, firstRunSystemIndex(1.0))
        assertEquals(1, firstRunSystemIndex(0.0))
        assertEquals(1, firstRunSystemIndex(Double.NaN))
        assertEquals(listOf("Metric System", "Imperial System"), FIRST_RUN_SYSTEMS)
    }
}

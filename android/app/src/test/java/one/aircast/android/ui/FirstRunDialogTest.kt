package one.aircast.android.ui

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
}

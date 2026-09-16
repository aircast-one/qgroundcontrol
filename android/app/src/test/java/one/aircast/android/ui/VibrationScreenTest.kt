package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class VibrationScreenTest {

    private val served = """
        {"available":true,"units":"m/s²","scaleMaximum":90,"warningLevel":30,"dangerLevel":60,
         "axes":[
           {"axis":"X","value":15.0,"fraction":0.1667,"severity":"normal"},
           {"axis":"Y","value":45.0,"fraction":0.5,"severity":"warning"},
           {"axis":"Z","value":75.0,"fraction":0.8333,"severity":"danger"}],
         "clipCounts":[0,3,12],"worst":"danger","clipping":true}
    """

    @Test
    fun `each severity keeps the word this screen has always shown`() {
        assertEquals("OK", severityLabel("normal"))
        assertEquals("Caution", severityLabel("warning"))
        assertEquals("High", severityLabel("danger"))
    }

    @Test
    fun `an absent severity is blank rather than a guess`() {
        assertEquals("", severityLabel(null))
        assertEquals("", severityLabel("something the core added later"))
    }

    @Test
    fun `the reading carries the axes and clip counts the core served`() {
        val reading = vibrationReading(JSONObject(served))!!

        assertEquals(listOf("X", "Y", "Z"), reading.axes.map { it.axis })
        assertEquals(listOf(15.0, 45.0, 75.0), reading.axes.map { it.value })
        assertEquals(listOf("normal", "warning", "danger"), reading.axes.map { it.severity })
        assertEquals(listOf(0, 3, 12), reading.clipCounts)
    }

    @Test
    fun `a null axis reads as absent, not as zero at the bottom of the scale`() {
        val reading = vibrationReading(
            JSONObject("""{"available":true,"units":"m/s²","scaleMaximum":90,
                "warningLevel":30,"dangerLevel":60,
                "axes":[{"axis":"X","value":null,"fraction":null,"severity":null}],
                "clipCounts":[]}"""),
        )!!

        assertNull(reading.axes[0].value)
        assertNull(reading.axes[0].severity)
        assertEquals(0f, reading.axes[0].fraction, 1e-6f)
    }

    @Test
    fun `an unavailable view is no reading at all`() {
        assertNull(vibrationReading(null))
        assertNull(vibrationReading(JSONObject("""{"available":false}""")))
    }

    @Test
    fun `some axes reported and some not is neither a reading nor a silence`() {
        val partial = JSONObject(
            """{"kind":"object","class":"Vibration","connected":true,"available":false,
               "silentReason":null,"silentText":null,"units":"m/s^2",
               "axes":[{"axis":"x","label":"X","value":12.0,"fraction":0.13,"severity":"normal"},
                       {"axis":"y","label":"Y","value":null,"fraction":null,"severity":null},
                       {"axis":"z","label":"Z","value":null,"fraction":null,"severity":null}]}""",
        )

        assertNull(
            "silentReason is set only when NO axis has a value, so a vehicle sending NaN in one " +
                "axis of a VIBRATION message leaves this null",
            silentState(partial),
        )
        assertNull(
            "available is all three, so the same view produces no reading - the screen used to " +
                "read reading!! after a silentState guard and threw on exactly this input",
            vibrationReading(partial),
        )
        assertEquals(
            "neither half covers it, so the screen has to; a guard that asks silentState alone " +
                "sends this input to the bars it has no reading for",
            PARTIAL_TITLE,
            vibrationEmptyState(partial, vibrationReading(partial))?.title,
        )
    }

    @Test
    fun `the scale and the caption are built from the core's own levels`() {
        assertEquals(listOf("90", "60", "30", "0"), scaleLabels(90.0, 30.0, 60.0))
        assertEquals("Under 30 healthy · 30-60 watch · over 60 unsafe", bandCaption(30.0, 60.0))
    }

    @Test
    fun `a blank unit leaves no empty brackets in the heading`() {
        assertEquals("Vibration (m/s²)", vibrationHeading("m/s²"))
        assertEquals("Vibration", vibrationHeading(""))
    }
}

class SilentStateTest {

    private fun view(reason: String, text: String) = JSONObject(
        """{"kind":"object","class":"Vibration","connected":true,"available":false,
           "silentReason":$reason,"silentText":$text,"axes":[],"units":"m/s^2"}""",
    )

    @Test
    fun `a reporting vehicle has no empty state`() {
        assertNull(silentState(view("null", "null")))
    }

    @Test
    fun `the title is the core's sentence and the body is this head's instruction`() {
        val gone = silentState(view("\"noVehicle\"", "\"No vehicle is connected.\""))!!

        assertEquals("No vehicle is connected.", gone.title)
        assertEquals("Connect a vehicle from the Fly view to see its vibration levels.", gone.body)
    }

    @Test
    fun `connected and silent gets the other instruction, chosen by token not by wording`() {
        val mute = silentState(
            view("\"notReported\"", "\"This vehicle reports no vibration measurements.\""),
        )!!

        assertEquals("This vehicle reports no vibration measurements.", mute.title)
        assertTrue(mute.body.startsWith("The autopilot has not sent a VIBRATION message"))
    }

    @Test
    fun `a token this head has never seen still gets a body`() {
        val odd = silentState(view("\"somethingNew\"", "\"Something new happened.\""))!!

        assertEquals("Something new happened.", odd.title)
        assertTrue(odd.body.isNotBlank())
    }

    @Test
    fun `no view at all is the disconnected case`() {
        val nothing = silentState(null)!!

        assertEquals("No vehicle connected", nothing.title)
    }
}

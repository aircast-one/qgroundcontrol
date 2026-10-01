package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class CalibrationViewTest {
    private val served = JSONObject(
        """{"kind":"object","class":"Calibration","connected":true,"inProgress":false,
            "busy":false,"showsSides":false,"nextEnabled":false,"cancelEnabled":false,
            "progress":0.0,"progressText":"0%","helpText":"","statusText":"",
            "accelNeeded":true,"compassNeeded":true,
            "needsAttention":"The accelerometer and compass both need calibrating.",
            "sides":[{"key":"Down","title":"Level","visible":false,"stage":"waiting","rotate":false}],
            "routines":[
              {"id":"accelerometer","title":"Accelerometer","invocation":"sensorsCal.calibrateAccel",
               "arguments":[false],"blocked":false,"enabled":true,
               "description":"Hold the vehicle in each orientation it asks for.","status":"Not calibrated","warning":""},
              {"id":"compass","title":"Compass","invocation":"sensorsCal.calibrateCompass",
               "arguments":[],"blocked":true,"enabled":false,
               "description":"Calibrate the accelerometer first.","status":"Calibrate the accelerometer first.","warning":""}]}""",
    )

    @Test
    fun `the routines come from the core, with the call it says to make`() {
        val state = calibrationState(served)!!

        assertEquals(listOf("accelerometer", "compass"), state.routines.map { it.id })
        assertEquals("sensorsCal.calibrateAccel", state.routines[0].invocation)
        assertEquals(listOf(false), state.routines[0].arguments)
        assertEquals(emptyList<Boolean>(), state.routines[1].arguments)
    }

    @Test
    fun `the accelerometer-first rule and each routine's status are the core's answer, not the head's`() {
        val state = calibrationState(served)!!

        assertFalse(state.routines[0].blocked)
        assertTrue(state.routines[1].blocked)
        assertEquals("Calibrate the accelerometer first.", state.routines[1].status)
        assertEquals("Not calibrated", state.routines[0].status)
    }

    @Test
    fun `a px4 routine opens with the dialog text qgc shows`() {
        val px4 = calibrationState(
            JSONObject(served.toString()).put("px4", true).put("settingsTitle", "Orientations"),
        )!!
        val gyro = px4.routines[0].copy(id = "gyro", dialogHelp = "For Gyroscope calibration you will need to place your vehicle on a surface and leave it still.")

        assertTrue(px4.px4)
        assertEquals("Orientations", px4.settingsTitle)
        assertEquals(gyro.dialogHelp, routineCopy(gyro).instruction)
    }

    @Test
    fun `the instruction is the core's dialog text, else its description`() {
        val state = calibrationState(served)!!

        assertEquals(
            "Rotate the vehicle until every side is done.",
            routineCopy(
                state.routines[1].copy(dialogHelp = "", description = "Rotate the vehicle until every side is done."),
            ).instruction,
        )
    }

    @Test
    fun `sides arrive with their stage rather than three separate flags`() {
        val running = calibrationState(
            JSONObject(
                """{"class":"Calibration","sides":[
                    {"key":"Down","title":"Level","visible":true,"stage":"done","rotate":false},
                    {"key":"Left","title":"Left side","visible":true,"stage":"inProgress","rotate":true}]}""",
            ),
        )!!

        assertEquals(listOf("done", "inProgress"), running.sides.map { it.stage })
        assertEquals(listOf(false, true), running.sides.map { it.rotate })
    }

    @Test
    fun `a payload that is not the calibration view yields nothing`() {
        assertNull(calibrationState(null))
        assertNull(calibrationState(JSONObject("{}")))
        assertNull(calibrationState(JSONObject("""{"class":"Vehicle"}""")))
    }

    @Test
    fun `a payload using invented names leaves the state empty rather than plausible`() {
        val invented = JSONObject(
            """{"class":"Calibration","calibrations":[{"id":"accelerometer"}],
                "orientations":[{"key":"Down"}],"inProgressNow":true}""",
        )
        val state = calibrationState(invented)!!

        assertEquals(emptyList<CalibrationRoutine>(), state.routines)
        assertEquals(emptyList<CalibrationSide>(), state.sides)
        assertFalse(state.inProgress)
    }
}

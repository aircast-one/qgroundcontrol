package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ArmingChecksTest {

    private fun warnings(checks: String) = JSONObject(
        """{"kind":"object","class":"VehicleWarnings","warnings":[],
           "armingBlocker":"Compass not calibrated","armingChecks":$checks}""",
    )

    @Test
    fun `every reason is listed, not only the one the banner had room for`() {
        val listed = armingChecks(
            warnings(
                """[{"message":"Compass not calibrated","description":"Calibrate the compass.","severity":"error"},
                    {"message":"GPS fix too poor","description":"Wait for more satellites.","severity":"error"},
                    {"message":"Battery below 20%","description":"","severity":"warning"}]""",
            ),
        )!!

        assertEquals(
            "armingBlocker is the FIRST error only, so an operator fixed one thing, tried again " +
                "and met the next - the core had sent all three the whole time",
            listOf("Compass not calibrated", "GPS fix too poor", "Battery below 20%"),
            listed.map { it.message },
        )
        assertEquals("warning", listed[2].severity)
    }

    @Test
    fun `nobody asked is not nothing is wrong`() {
        assertNull(
            "warnings.rs serves null when the firmware has never sent a report, because canArm " +
                "initialises true and supported false - a vehicle that passed every check and one " +
                "that was never asked would otherwise read identically",
            armingChecks(warnings("null")),
        )
        assertEquals(emptyList<ArmingCheck>(), armingChecks(warnings("[]")))
    }

    @Test
    fun `an entry with no message is not a reason`() {
        assertEquals(
            emptyList<ArmingCheck>(),
            armingChecks(warnings("""[{"message":"","description":"","severity":"error"}]""")),
        )
    }
}

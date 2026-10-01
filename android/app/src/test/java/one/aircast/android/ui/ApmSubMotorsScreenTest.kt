package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ApmSubMotorsScreenTest {
    @Test
    fun `motors, their directions and the test gate come from the core`() {
        val read = subMotors(JSONObject("""{"available":true,"armed":true,"detecting":false,"canRunManualTest":true,
            "motors":[{"motor":1,"reversed":false},{"motor":2,"reversed":true}],"warning":"w","offersAutoDetect":true,"autoDetectHelp":"h","detectionMessages":"Thruster 1 ok\n"}"""))!!
        assertEquals(listOf(SubMotor(1, false), SubMotor(2, true)), read.motors)
        assertTrue(read.canRunManualTest)
        assertTrue(read.offersAutoDetect)
        assertEquals("Thruster 1 ok\n", read.detectionMessages)
        assertNull(subMotors(JSONObject("""{"available":false}""")))
    }
}

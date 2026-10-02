package one.aircast.android

import org.json.JSONObject
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class GamepadStreamsTest {
    @Test
    fun theGamepadStreamsOnlyWhileItDrivesAVehicleOrIsBeingCalibrated() {
        assertTrue(streams(JSONObject("""{"vehicle":true,"enabled":true,"calibration":null}""")))
        assertFalse(streams(JSONObject("""{"vehicle":true,"enabled":false,"calibration":null}""")))
        assertFalse(streams(JSONObject("""{"vehicle":false,"enabled":true,"calibration":null}""")))
        assertTrue(streams(JSONObject("""{"vehicle":false,"enabled":false,"calibration":{"step":1}}""")))
        assertFalse(streams(null))
    }
}

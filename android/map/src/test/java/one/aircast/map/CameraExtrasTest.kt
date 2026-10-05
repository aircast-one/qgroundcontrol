package one.aircast.map

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CameraExtrasTest {
    @Test
    fun `the camera section's interval, mode and gimbal come from the core`() {
        val extras = cameraExtras(JSONObject("""{"available":true,"intervalTime":{"value":4.0},"intervalDistance":null,"cameraModeSupported":true,"commandsMode":true,"cameraMode":{"choice":1},"commandsGimbal":true,"gimbalPitch":{"value":-45.0},"gimbalYaw":null}"""))!!
        assertEquals(4.0, extras.intervalTime!!, 0.0)
        assertNull(extras.intervalDistance)
        assertEquals(1, extras.mode)
        assertEquals(-45.0, extras.pitch, 0.0)
        assertEquals(0.0, extras.yaw, 0.0)
        assertNull(cameraExtras(JSONObject("""{"available":false}""")))
        assertEquals("10", trimmedNumber(10.0))
    }

    @Test
    fun `the gimbal angles slide over the fact's user range like CameraSection's FactTextFieldSlider`() {
        val extras = cameraExtras(JSONObject("""{"available":true,"commandsGimbal":true,"gimbalPitch":{"value":45.0,"slider":{"from":90.0,"to":0.0,"decimals":0}},"gimbalYaw":{"value":0.0,"slider":{"from":-180.0,"to":180.0,"decimals":0}}}"""))!!
        assertEquals(0.0..90.0, extras.pitchRange)
        assertEquals(-180.0..180.0, extras.yawRange)
        assertNull(cameraExtras(JSONObject("""{"available":true,"gimbalPitch":{"value":-45.0}}"""))!!.pitchRange)
    }

    @Test
    fun `the photo distance is labelled in the units its fact is cooked to`() {
        val feet = cameraExtras(JSONObject("""{"available":true,"intervalDistance":{"value":100.0,"units":"ft"}}"""))!!
        assertEquals(100.0, feet.intervalDistance!!, 0.0)
        assertEquals("ft", feet.distanceUnits)
        assertEquals("m", cameraExtras(JSONObject("""{"available":true,"intervalDistance":{"value":3.0}}"""))!!.distanceUnits)
    }
}

package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ItemCameraTest {

    private fun view(
        available: Boolean = true,
        commandsGimbal: Boolean = true,
        action: String = """{"value":1,"text":"Take photo","units":""}""",
        pitch: String = """{"value":-45.0,"text":"-45.0","units":"deg"}""",
        yaw: String = """{"value":0.0,"text":"0.0","units":"deg"}""",
    ) = JSONObject(
        """{"kind":"object","class":"ItemCamera","index":1,"available":$available,
           "commandsGimbal":$commandsGimbal,"cameraAction":$action,
           "gimbalPitch":$pitch,"gimbalYaw":$yaw}""",
    )

    @Test
    fun `an item with no camera section says nothing`() {
        assertNull(itemCameraText(null))
        assertNull(itemCameraText(view(available = false)))
    }

    @Test
    fun `an item that commands the gimbal states the action and both angles`() {
        assertEquals("Take photo · gimbal -45.0 deg / 0.0 deg", itemCameraText(view()))
    }

    @Test
    fun `angles are withheld for an item that does not command the gimbal`() {
        val untouched = view(commandsGimbal = false, pitch = "null", yaw = "null")

        assertEquals("Take photo", itemCameraText(untouched))
    }

    @Test
    fun `an item that touches neither is a blank line rather than an empty bullet`() {
        val nothing = view(commandsGimbal = false, action = "null", pitch = "null", yaw = "null")

        assertNull(itemCameraText(nothing))
    }

    @Test
    fun `a bare number is not a camera action and is not drawn as one`() {
        val unlabelled = view(
            commandsGimbal = false,
            action = """{"value":0,"text":"0.000","units":""}""",
            pitch = "null",
            yaw = "null",
        )

        assertNull(itemCameraText(unlabelled))
        assertEquals("", namedAction("0.000"))
        assertEquals("", namedAction("-1"))
        assertEquals("Take photo", namedAction("Take photo"))
    }

    @Test
    fun `a number beside a gimbal angle still leaves the angle`() {
        val mixed = view(action = """{"value":0,"text":"0.000","units":""}""")

        assertEquals("gimbal -45.0 deg / 0.0 deg", itemCameraText(mixed))
    }

    @Test
    fun `a measure with no units is not given a trailing space`() {
        val unitless = view(
            commandsGimbal = false,
            action = """{"value":1,"text":"Take photo","units":null}""",
            pitch = "null",
            yaw = "null",
        )

        assertEquals("Take photo", itemCameraText(unitless))
    }
}

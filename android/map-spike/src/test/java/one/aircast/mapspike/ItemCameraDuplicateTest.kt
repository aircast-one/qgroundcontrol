package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ItemCameraDuplicateTest {

    private fun camera(commandsGimbal: Boolean) = JSONObject(
        """{"available":true,"commandsGimbal":$commandsGimbal,
           "cameraAction":{"choices":["No change","Take photo"],"choice":0,"text":"No change"},
           "gimbalPitch":{"text":"-90 deg"},"gimbalYaw":{"text":"45 deg"}}""",
    )

    @Test
    fun `the summary is dropped when the picker beside it already says the same word`() {
        val view = camera(commandsGimbal = false)
        assertEquals("No change", itemCameraText(view))
        assertNull(
            "the picker button renders labels[chosen] and the summary renders the same string, " +
                "so a waypoint with no gimbal drew 'No change' twice side by side",
            itemCameraTextBeside(view, pickerLabel = "No change"),
        )
    }

    @Test
    fun `the summary stays when it carries the gimbal the picker cannot show`() {
        val view = camera(commandsGimbal = true)
        assertEquals(
            "the picker only ever names the action, so the gimbal half is the summary's reason " +
                "to exist and must survive",
            "No change · gimbal -90 deg / 45 deg",
            itemCameraTextBeside(view, pickerLabel = "No change"),
        )
    }

    @Test
    fun `with no picker at all the summary is whatever it was`() {
        assertEquals("No change", itemCameraTextBeside(camera(commandsGimbal = false), pickerLabel = null))
    }
}

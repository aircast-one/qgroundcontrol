package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class CameraSettingsSectionTest {
    private val view = JSONObject(
        """{"state":"ready","activeSettings":["CAM_EV","CAM_WBMODE","CAM_FLICKER","CAM_NAME"],"parameters":[
            {"name":"CAM_WBMODE","description":"White Balance","readOnly":false,"isBool":false,"value":1,"selected":1,"options":[{"label":"Auto","value":0},{"label":"Sunny","value":1}]},
            {"name":"CAM_EV","description":"Exposure Compensation","readOnly":false,"isBool":false,"value":0.5,"min":-2.0,"max":2.0,"step":0.5,"options":[]},
            {"name":"CAM_FLICKER","description":"Flicker","readOnly":true,"isBool":true,"value":1,"options":[]},
            {"name":"CAM_NAME","description":"Name","readOnly":false,"isBool":false,"value":"E90","options":[]},
            {"name":"CAM_HIDDEN","description":"Hidden","readOnly":false,"isBool":false,"value":0,"options":[]}]}""",
    )

    @Test
    fun `only the active settings show, in the camera's order, each with qgc's control`() {
        val shown = cameraSettings(view)
        assertEquals(listOf("CAM_EV", "CAM_WBMODE", "CAM_FLICKER", "CAM_NAME"), shown.map { it.name })
        assertEquals(CameraSettingControl.Range(-2.0, 2.0, 0.5, 0.5), shown[0].control)
        assertEquals(1, (shown[1].control as CameraSettingControl.Choice).selected)
        assertEquals(CameraSettingControl.Toggle(true), shown[2].control)
        assertTrue(shown[2].readOnly)
        assertEquals(CameraSettingControl.Entry("E90"), shown[3].control)
    }

    @Test
    fun `nothing shows until the definition is loaded`() {
        assertEquals(emptyList<CameraSetting>(), cameraSettings(JSONObject("""{"state":"fetching"}""")))
        assertEquals(emptyList<CameraSetting>(), cameraSettings(null))
    }
}

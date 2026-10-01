package one.aircast.android.ui

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class CameraCalcSectionTest {
    private fun control(suffix: String) = JSONObject(
        """{"class":"Control","path":"plan.missionController.visualItems.1.$suffix","name":"$suffix","pathSuffix":"$suffix","value":1,"valueString":"1"}""",
    )

    private fun view(brand: String, custom: Boolean, byDistance: Boolean) = JSONObject().put(
        "camera",
        JSONObject()
            .put("brand", brand).put("manualName", "Manual (no camera specs)").put("custom", custom).put("valueSetIsDistance", byDistance)
            .put("facts", JSONArray(listOf("cameraCalc.sensorWidth", "cameraCalc.distanceToSurface", "cameraCalc.imageDensity", "cameraCalc.frontalOverlap", "cameraCalc.sideOverlap").map(::control))),
    )

    private fun shown(brand: String, custom: Boolean, byDistance: Boolean) =
        shownCameraFacts(cameraCalc(view(brand, custom, byDistance))!!).map { it.path.substringAfter("visualItems.1.") }

    @Test
    fun `the grid shows the figure that sets the other, as CameraCalcGrid does`() {
        assertEquals(listOf("cameraCalc.distanceToSurface", "cameraCalc.frontalOverlap", "cameraCalc.sideOverlap"), shown("Sony", false, true))
        assertEquals(listOf("cameraCalc.imageDensity", "cameraCalc.frontalOverlap", "cameraCalc.sideOverlap"), shown("Sony", false, false))
        assertEquals(listOf("cameraCalc.sensorWidth", "cameraCalc.distanceToSurface", "cameraCalc.frontalOverlap", "cameraCalc.sideOverlap"), shown("Custom Camera", true, true))
        assertEquals(listOf("cameraCalc.distanceToSurface"), shown("Manual (no camera specs)", false, false))
    }
}

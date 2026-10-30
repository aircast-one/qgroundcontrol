package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

class DeviceDetectionTest {
    private val served =
        """
        {
          "status": {"level": 3, "text": "sim · Sim targets · on the CPU"},
          "unavailable": true,
          "cameras": [
            {"name": "front", "on": true, "status": {"level": 0, "text": "live"}},
            {"name": "belly", "on": false, "status": {"level": 3, "text": "off"}}
          ],
          "models": [
            {"name": "sim", "path": "/m/sim.onnx", "about": "Sim targets", "size": "12 MB", "classes": {"level": 3, "text": "3 classes"}, "engine": "CPU", "active": true},
            {"name": "visdrone", "path": "/m/visdrone.hef", "size": "9 MB", "classes": {"level": 1, "text": "no labels"}, "engine": "Hailo-8", "active": false}
          ]
        }
        """

    @Test
    fun `reads the detection page the device serves`() {
        val view = parseDetection(JSONObject(served))!!
        assertEquals(DeviceBadge(3, "sim · Sim targets · on the CPU"), view.status)
        assertTrue(view.unavailable)
        assertEquals(listOf(DetectCamera("front", true, DeviceBadge(0, "live")), DetectCamera("belly", false, DeviceBadge(3, "off"))), view.cameras)
        assertEquals(DetectModel("sim", "/m/sim.onnx", "Sim targets", "12 MB", DeviceBadge(3, "3 classes"), "CPU", true), view.models.first())
        assertEquals(DetectModel("visdrone", "/m/visdrone.hef", "", "9 MB", DeviceBadge(1, NO_LABELS), "Hailo-8", false), view.models.last())
        assertTrue(detecting(view))
    }

    @Test
    fun `a device with no camera detecting says so, and an answer without a status is not this page`() {
        val view = parseDetection(JSONObject(served).put("cameras", org.json.JSONArray("""[{"name": "front", "on": false, "status": {"level": 3, "text": "off"}}]""")))!!
        assertFalse(detecting(view))
        assertNull(parseDetection(JSONObject()))
    }

    @Test
    fun `an upload sends the model and only the extras that were given`() {
        val hef = File("m.hef")
        val labels = File("m.labels")
        assertEquals(listOf(DevicePart("hef", hef)), modelFields(ModelUpload(hef, null, "  ", use = false)))
        assertEquals(
            listOf(DevicePart("hef", hef), DevicePart("labels", labels), DevicePart("about", value = "Aerial"), DevicePart("use", value = "1")),
            modelFields(ModelUpload(hef, labels, " Aerial ", use = true)),
        )
    }

    @Test
    fun `an upload says whether it switched, and whether any camera is detecting on it`() {
        assertEquals(UploadOutcome(null, "Model uploaded and switched — detection restarts on it."), uploadMessage(switched = true, use = true, cameraDetecting = true))
        assertEquals(
            UploadOutcome(null, "Model uploaded and switched — no camera is detecting yet. Turn it on for a camera on this page."),
            uploadMessage(switched = true, use = true, cameraDetecting = false),
        )
        assertEquals(
            UploadOutcome(DeviceRefusal("Model uploaded, but switching to it failed — pick it from the list.", ""), ""),
            uploadMessage(switched = false, use = true, cameraDetecting = true),
        )
        assertEquals(UploadOutcome(null, "Model uploaded — use it from the list when you want it."), uploadMessage(switched = false, use = false, cameraDetecting = true))
    }

    @Test
    fun `a model name is escaped into its path and its labels path`() {
        assertEquals("/api/detect/models/aerial%20v2", modelPath("aerial v2"))
        assertEquals("/api/detect/models/aerial%20v2/labels", labelsPath("aerial v2"))
    }
}

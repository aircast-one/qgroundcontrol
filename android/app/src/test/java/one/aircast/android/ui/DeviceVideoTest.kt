package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

class DeviceVideoTest {
    private val served =
        """
        {
          "radio": {"status": {"level": 0, "text": "on air"}, "channel": "165", "region": "BO", "txPower": "", "details": "UDP 5602"},
          "sharing": {"status": {"level": 1, "text": "finish setup"}, "stage": "deploy", "deployUrl": "https://deploy.workers.cloudflare.com/?url=x", "signKey": "sk", "deviceToken": "dt"},
          "detection": {
            "status": {"level": 3, "text": "visdrone · Aerial"},
            "unavailable": true,
            "models": [{"name": "visdrone", "path": "/m/visdrone.hef", "about": "Aerial", "size": "12 MB", "classes": {"level": 3, "text": "10 classes"}, "active": true}]
          },
          "engineering": {"title": "Engineering", "href": "/settings/engineering", "icon": "server", "summary": "mediamtx"}
        }
        """

    @Test
    fun `reads the video page the device serves`() {
        val view = parseVideo(JSONObject(served))
        assertEquals(RadioSettings(DeviceBadge(0, "on air"), "165", "BO", "", "UDP 5602"), view?.radio)
        assertEquals(SharingStage.Deploy, view?.sharing?.stage)
        assertEquals("sk", view?.sharing?.signKey)
        assertEquals("dt", view?.sharing?.deviceToken)
        assertEquals("", view?.sharing?.baseUrl)
        assertTrue(view?.detection?.unavailable == true)
        assertEquals("visdrone", view?.detection?.models?.first()?.name)
        assertTrue(view?.detection?.models?.first()?.active == true)
        assertEquals(DeviceBadge(3, "10 classes"), view?.detection?.models?.first()?.classes)
        assertEquals(DevicePageLink("Engineering", "/settings/engineering", "mediamtx"), view?.engineering)
    }

    @Test
    fun `a device with no radio has no radio card`() {
        assertNull(parseVideo(JSONObject(served).apply { remove("radio") })?.radio)
        assertNull(parseVideo(JSONObject().put("radio", JSONObject(served).getJSONObject("radio"))))
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
    fun `a switch that did not happen after upload is said, not swallowed`() {
        assertEquals(UploadOutcome(null, "Model uploaded and switched — detection restarts on it."), uploadMessage(switched = true, use = true))
        assertTrue(uploadMessage(switched = false, use = true).refusal?.message.orEmpty().contains("switching to it failed"))
        assertNull(uploadMessage(switched = false, use = false).refusal)
    }

    @Test
    fun `a model name is escaped into its path`() {
        assertEquals("/api/detect/models/aerial%20v2", modelPath("aerial v2"))
    }
}

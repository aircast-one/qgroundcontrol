package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CamerasTest {
    private val view = JSONObject(
        """
        {"kind":"object","class":"Cameras","readable":true,"reason":null,"active":2,
         "pip":{"enabled":false,"slot":0},
         "cameras":[
           {"slot":0,"stored":0,"title":"Front gimbal","short":"Front","name":"Front gimbal","source":"RTSP Video Stream","url":"rtsp://10.0.0.5:8554/front","problem":null,"fromDrone":false,"active":false,"status":"connecting"},
           {"slot":1,"stored":1,"title":"Camera 2","short":"Cam 2","name":"","source":"Back Camera","url":"","problem":"This kind of camera cannot show video in this app.","fromDrone":false,"active":false,"status":"bogus"},
           {"slot":2,"stored":null,"title":"SIYI A8","short":"SIYI","name":"SIYI A8","source":"UDP h.264 Video Stream","url":"0.0.0.0:5600","problem":null,"fromDrone":true,"active":true,"status":"live"}],
         "kinds":[
           {"raw":"Front Camera","label":"Front Camera","group":"This device","needsUrl":false,"hint":"","description":"This device's front camera"},
           {"raw":"RTSP Video Stream","label":"RTSP Video Stream","group":"Video streams","needsUrl":true,"hint":"rtsp://192.168.1.10:8554/live","description":"An rtsp:// address from an IP camera or video server"},
           {"raw":"UDP h.265 Video Stream","label":"UDP h.265 Video Stream","group":"Video streams","needsUrl":true,"hint":"0.0.0.0:5600","defaultAddress":"0.0.0.0:5600"},
           {"raw":"Herelink Hotspot","label":"Herelink Hotspot","group":"Vehicle and radio presets","needsUrl":false,"hint":"","description":"rtsp://192.168.43.1:8554/fpv_stream"},
           {"raw":"Back Camera","label":"Back Camera","group":"This device","needsUrl":false,"hint":""}]}
        """.trimIndent(),
    )

    @Test
    fun `every camera reads as one list, and only the operator's own can be edited`() {
        val reading = camerasReading(view)!!
        assertEquals(3, reading.cameras.size)
        assertEquals(listOf(0, 1), reading.stored.map { it.stored })
        assertNull(reading.cameras[2].stored)
        assertEquals(true, reading.cameras[2].fromDrone)
        assertEquals("This kind of camera cannot show video in this app.", reading.cameras[1].problem)
        assertNull(reading.cameras[0].problem)
        assertEquals("rtsp://192.168.1.10:8554/live", reading.kinds[1].hint)
        assertEquals("An rtsp:// address from an IP camera or video server", reading.kinds[1].description)
        assertEquals("0.0.0.0:5600", reading.kinds[2].defaultAddress)
        assertNull(camerasReading(JSONObject("""{"class":"Video"}""")))
    }

    @Test
    fun `each camera carries its signal, its short name, and the picture-in-picture pick`() {
        val reading = camerasReading(view)!!
        assertEquals(listOf(CameraStatus.Connecting, CameraStatus.Idle, CameraStatus.Live), reading.cameras.map { it.status })
        assertEquals(listOf("Front", "Cam 2", "SIYI"), reading.cameras.map { it.short })
        assertEquals(CameraPip(enabled = false, slot = 0), reading.pip)
        assertEquals(CameraPip(enabled = true, slot = null), camerasReading(JSONObject("""{"class":"Cameras","pip":{"enabled":true,"slot":null}}"""))!!.pip)
        assertEquals(CameraStatus.NoSignal, camerasReading(JSONObject("""{"class":"Cameras","cameras":[{"status":"noSignal"}]}"""))!!.cameras.single().status)
    }

    @Test
    fun `a row names its type beside an address that does not spell it, or that the drone brought it`() {
        val cameras = camerasReading(view)!!.cameras
        assertEquals(listOf("rtsp://10.0.0.5:8554/front", "Back camera", "From the drone"), cameras.map(::cameraDetail))
        assertEquals("UDP h.265 · 0.0.0.0:5600", cameraDetail(cameras[0].copy(source = "UDP h.265 Video Stream", url = "0.0.0.0:5600")))
    }

    @Test
    fun `the type list puts streams first, then presets, then this phone, never a fixed camera already listed`() {
        val reading = camerasReading(view)!!
        assertEquals(listOf("RTSP Video Stream", "UDP h.265 Video Stream", "Herelink Hotspot", "Front Camera"), addableKinds(reading).map { it.raw })
        assertEquals("an edit may keep the fixed camera it already is", true, addableKinds(reading, keeping = "Back Camera").any { it.raw == "Back Camera" })
        assertEquals(listOf("RTSP", "UDP h.265", "Herelink Hotspot", "Front camera"), addableKinds(reading).map(::kindTitle))
        assertEquals(emptyList<CameraKind>(), addableKinds(null))
    }

    @Test
    fun `the synthetic view is offered by its own name`() {
        val synthetic = CameraKind(raw = SYNTHETIC_SOURCE, label = SYNTHETIC_SOURCE, group = "This device", needsUrl = false, hint = "")
        assertEquals("Synthetic view", kindTitle(synthetic))
        assertEquals("it is added under its own name rather than as Camera N", "Synthetic view", addedName(synthetic))
        assertEquals("", addedName(synthetic.copy(raw = "Front Camera")))
    }

    private val udp = CameraKind(raw = "UDP h.265 Video Stream", label = "UDP h.265 Video Stream", group = "Video streams", needsUrl = true, hint = "0.0.0.0:5600", defaultAddress = "0.0.0.0:5600")
    private val rtsp = CameraKind(raw = "RTSP Video Stream", label = "RTSP Video Stream", group = "Video streams", needsUrl = true, hint = "rtsp://192.168.1.10:8554/live")
    private val hotspot = CameraKind(raw = "Herelink Hotspot", label = "Herelink Hotspot", group = "Vehicle and radio presets", needsUrl = false, hint = "")

    @Test
    fun `picking a type starts the address where most radios send, and keeps one already typed`() {
        val udpDraft = pickedKind(NEW_CAMERA_DRAFT, udp)
        assertEquals(CameraDraft(stored = null, title = "", name = "", source = udp.raw, url = "0.0.0.0:5600"), udpDraft)
        assertEquals("an address to dial starts empty", "", pickedKind(NEW_CAMERA_DRAFT, rtsp).url)
        assertEquals("switching type keeps the typed address", "0.0.0.0:5600", pickedKind(udpDraft.copy(choosingKind = true), rtsp).url)
        val preset = pickedKind(udpDraft, hotspot)
        assertEquals("a preset has no address of its own to keep", Pair(false, ""), preset.needsUrl to preset.url)
    }

    @Test
    fun `a new camera is added with the type picked, an edit updates its slot`() {
        assertEquals(CameraSave.Add("Belly", udp.raw, "0.0.0.0:5600"), cameraSave(pickedKind(NEW_CAMERA_DRAFT, udp).copy(name = "Belly")))
        val front = camerasReading(view)!!.cameras[0]
        val edited = pickedKind(editDraft(front, camerasReading(view)!!.kinds)!!, udp).copy(url = "0.0.0.0:5601")
        assertEquals(CameraSave.Update(0, "Front gimbal", udp.raw, "0.0.0.0:5601"), cameraSave(edited))
        assertEquals("a drone camera is not the operator's to edit", null, editDraft(camerasReading(view)!!.cameras[2], emptyList()))
    }

    @Test
    fun `a refused save lands in the sheet as it is now, and never reopens a sheet the operator closed`() {
        val typedWhileSaving = pickedKind(NEW_CAMERA_DRAFT, udp).copy(name = "Belly cam")
        assertEquals(typedWhileSaving.copy(refusal = "No"), refusedDraft(typedWhileSaving, "No"))
        assertNull("a cancelled sheet stays closed", refusedDraft(null, "No"))
        assertNull("a save that went through closes the sheet", refusedDraft(typedWhileSaving, null))
    }

    @Test
    fun `the video sources page is listed under Transmission though it has no settings of its own`() {
        val pages = settingsPages(JSONObject("""{"pages":[{"title":"Video sources","showsVideoSources":true,"sections":[]}]}"""))
        assertEquals(listOf(VIDEO_SOURCES_PAGE), pages.map { it.title })
        assertEquals(SettingsGroup.Transmission, pageLook(VIDEO_SOURCES_PAGE).group)
    }

    @Test
    fun `the settings index names the camera on screen and how many there are`() {
        assertEquals("SIYI A8 · 3 cameras", camerasGlance(camerasReading(view)))
        assertEquals("", camerasGlance(null))
        assertEquals("RTSP", kindLabel("RTSP Video Stream"))
    }
}

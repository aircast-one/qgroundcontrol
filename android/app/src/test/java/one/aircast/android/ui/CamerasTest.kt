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
           {"raw":"RTSP Video Stream","label":"RTSP Video Stream","group":"Video streams","needsUrl":true,"hint":"rtsp://192.168.1.10:8554/live"},
           {"raw":"Herelink Hotspot","label":"Herelink Hotspot","group":"Vehicle and radio presets","needsUrl":false,"hint":""},
           {"raw":"Back Camera","label":"Back Camera","group":"This device","needsUrl":false,"hint":""},
           {"raw":"Front Camera","label":"Front Camera","group":"This device","needsUrl":false,"hint":""}]}
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
        assertEquals("rtsp://192.168.1.10:8554/live", reading.kinds.first().hint)
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
    fun `a row names its address, or that the drone brought it`() {
        val cameras = camerasReading(view)!!.cameras
        assertEquals(listOf("rtsp://10.0.0.5:8554/front", "Back camera", "From the drone"), cameras.map(::cameraDetail))
    }

    @Test
    fun `the add sheet offers this phone's cameras first, then presets, never one already listed`() {
        val reading = camerasReading(view)!!
        assertEquals(listOf("Front Camera", "Herelink Hotspot"), otherSources(reading).map { it.raw })
        assertEquals(listOf("This phone's front camera", "Herelink Hotspot"), otherSources(reading).map(::otherSourceLabel))
        assertEquals(emptyList<CameraKind>(), otherSources(null))
    }

    @Test
    fun `an address is classified by the core, and an ambiguous one lets the operator pick`() {
        val ambiguous = cameraGuess(
            JSONObject(
                """{"ok":true,"address":"0.0.0.0:5600","kind":"UDP h.264 Video Stream","choices":["UDP h.264 Video Stream","UDP h.265 Video Stream","MPEG-TS Video Stream","TCP-MPEG2 Video Stream"],"ambiguous":true,"problem":null}""",
            ),
        )!!
        assertEquals("UDP h.264 Video Stream", ambiguous.kind)
        assertEquals(4, ambiguous.choices.size)
        assertEquals("Which kind of stream is it?", guessText(ambiguous))
        assertEquals("the default choice when nothing is picked", "UDP h.264 Video Stream", chosenKind(ambiguous, null, ""))
        assertEquals("UDP h.265 Video Stream", chosenKind(ambiguous, "UDP h.265 Video Stream", ""))
        assertEquals("a pick the address does not allow falls back to the guess", "UDP h.264 Video Stream", chosenKind(ambiguous, "RTSP Video Stream", ""))

        val rtsp = cameraGuess(JSONObject("""{"ok":true,"address":"rtsp://cam/live","kind":"RTSP Video Stream","choices":["RTSP Video Stream"],"ambiguous":false,"problem":null}"""))!!
        assertEquals("RTSP stream", guessText(rtsp))

        val refused = cameraGuess(JSONObject("""{"ok":true,"address":"ftp://x","kind":null,"choices":[],"ambiguous":false,"problem":"Start the address with rtsp://, http://, udp:// or tcp://, or type it as host:port."}"""))!!
        assertNull(refused.kind)
        assertEquals(refused.problem, guessText(refused))

        assertEquals("no guess yet leaves the core to infer", "", chosenKind(null, null, ""))
        assertEquals("", guessText(null))
        assertNull(cameraGuess(JSONObject("""{"ok":false,"reason":"no"}""")))
        assertNull(cameraGuess(null))
    }

    @Test
    fun `renaming a working camera keeps its kind even where its address alone reads as another`() {
        val webrtc = "WebRTC (WHEP) Video Stream"
        val hostPort = cameraGuess(
            JSONObject(
                """{"ok":true,"address":"192.168.1.10:8889","kind":"UDP h.264 Video Stream","choices":["UDP h.264 Video Stream","UDP h.265 Video Stream","MPEG-TS Video Stream","TCP-MPEG2 Video Stream"],"ambiguous":true,"problem":null}""",
            ),
        )
        val kept = keptGuess(hostPort, webrtc, "192.168.1.10:8889", "192.168.1.10:8889")!!
        assertEquals(CameraGuess(webrtc, listOf(webrtc), ambiguous = false, problem = null), kept)
        assertEquals(webrtc, chosenKind(kept, webrtc, webrtc))

        val unknown = cameraGuess(JSONObject("""{"ok":true,"address":"cam:8889/whep","kind":null,"choices":[],"ambiguous":false,"problem":"Start the address with rtsp://, http://, udp:// or tcp://, or type it as host:port."}"""))
        assertEquals("an address the classifier cannot read still saves under its kind", webrtc, keptGuess(unknown, webrtc, "cam:8889/whep ", "cam:8889/whep")!!.kind)

        assertEquals("a changed address is guessed afresh", hostPort, keptGuess(hostPort, webrtc, "192.168.1.10:8890", "192.168.1.10:8889"))
        assertEquals("a broken or new camera has nothing to keep", hostPort, keptGuess(hostPort, webrtc, "192.168.1.10:8889", null))
        assertEquals("a kind among the choices stays a choice", hostPort, keptGuess(hostPort, "MPEG-TS Video Stream", "192.168.1.10:8889", "192.168.1.10:8889"))
        assertNull(keptGuess(null, webrtc, "192.168.1.10:8889", "192.168.1.10:8889"))
    }

    private val rtspGuess = CameraGuess("RTSP Video Stream", listOf("RTSP Video Stream"), ambiguous = false, problem = null)
    private val unreadable = CameraGuess(null, emptyList(), ambiguous = false, problem = "Start the address with rtsp://, http://, udp:// or tcp://, or type it as host:port.")
    private val newDraft = CameraDraft(stored = null, title = "", name = "Belly", source = "", url = "rtsp://cam/live")
    private val editDraft = CameraDraft(stored = 0, title = "Front gimbal", name = "Front gimbal", source = "RTSP Video Stream", url = "ftp://cam", picked = "RTSP Video Stream", kept = "rtsp://10.0.0.5:8554/front")

    @Test
    fun `saving before the typed address was classified classifies it on the spot`() {
        val asked = mutableListOf<String>()
        val save = cameraSave(newDraft, shown = null) { address -> asked += address; rtspGuess }
        assertEquals(listOf("rtsp://cam/live"), asked)
        assertEquals(CameraSave.Add("Belly", "RTSP Video Stream", "rtsp://cam/live"), save)
    }

    @Test
    fun `a guess already on screen is saved as shown, without asking again`() {
        val save = cameraSave(newDraft, rtspGuess) { error("classified twice") }
        assertEquals(CameraSave.Add("Belly", "RTSP Video Stream", "rtsp://cam/live"), save)
    }

    @Test
    fun `an edit to an address the core cannot read is refused before anything is written`() {
        assertEquals(CameraSave.Refused(unreadable.problem!!), cameraSave(editDraft, unreadable) { error("classified twice") })
        assertEquals(CameraSave.Refused(unreadable.problem!!), cameraSave(editDraft, shown = null) { unreadable })
    }

    @Test
    fun `a new address the classifier cannot read is still handed to the core, which names the problem`() {
        assertEquals(CameraSave.Add("Belly", "", "rtsp://cam/live"), cameraSave(newDraft, unreadable) { error("classified twice") })
    }

    @Test
    fun `an edit keeps the camera's kind for its old address, and a camera with no address is renamed without classifying`() {
        val kept = editDraft.copy(url = "rtsp://10.0.0.5:8554/front")
        assertEquals(CameraSave.Update(0, "Front gimbal", "RTSP Video Stream", "rtsp://10.0.0.5:8554/front"), cameraSave(kept, shown = null) { unreadable })
        val phone = CameraDraft(stored = 1, title = "Camera 2", name = "Phone", source = "Back Camera", url = "", needsUrl = false)
        assertEquals(CameraSave.Update(1, "Phone", "Back Camera", ""), cameraSave(phone, shown = null) { error("nothing to classify") })
    }

    @Test
    fun `a refused save lands in the sheet as it is now, and never reopens a sheet the operator closed`() {
        val typedWhileSaving = newDraft.copy(name = "Belly cam")
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

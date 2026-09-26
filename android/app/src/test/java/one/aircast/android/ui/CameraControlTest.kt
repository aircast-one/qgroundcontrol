package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class CameraControlTest {

    private fun camera(json: String) = cameraReading(JSONObject(json))

    @Test
    fun `a photo camera offers a photo shutter`() {
        val shutter = shutterFor(
            camera("""{"present":true,"hasModes":true,"modeText":"Photo","mode":0,
                "modeKnown":true,"canPhoto":true,"canRecord":true,
                "isRecording":false,"isTakingPhoto":false}""")!!,
        )!!

        assertEquals("Take Photo", shutter.label)
        assertTrue(shutter.enabled)
        assertFalse(shutter.recording)
    }

    @Test
    fun `a photo already in progress disables the shutter rather than queueing another`() {
        val shutter = shutterFor(
            camera("""{"present":true,"hasModes":true,"modeText":"Photo","mode":0,
                "modeKnown":true,"canPhoto":true,"canRecord":true,
                "isRecording":false,"isTakingPhoto":true}""")!!,
        )!!

        assertFalse(shutter.enabled)
    }

    @Test
    fun `video mode records and then stops`() {
        val idle = shutterFor(
            camera("""{"present":true,"hasModes":true,"modeText":"Video","mode":1,
                "modeKnown":true,"canPhoto":true,"canRecord":true,
                "isRecording":false,"isTakingPhoto":false}""")!!,
        )!!
        val running = shutterFor(
            camera("""{"present":true,"hasModes":true,"modeText":"Video","mode":1,
                "modeKnown":true,"canPhoto":true,"canRecord":true,
                "isRecording":true,"isTakingPhoto":false}""")!!,
        )!!

        assertEquals("Record", idle.label)
        assertEquals("Stop", running.label)
        assertTrue(running.recording)
    }

    @Test
    fun `a camera in a mode it cannot do offers no shutter`() {
        assertNull(
            shutterFor(
                camera("""{"present":true,"hasModes":true,"modeText":"Video","mode":1,
                    "modeKnown":true,"canPhoto":true,"canRecord":false,
                    "isRecording":false,"isTakingPhoto":false}""")!!,
            ),
        )
    }

    @Test
    fun `an unknown mode is not treated as photo`() {
        val unknown = camera("""{"present":true,"hasModes":true,"modeText":"Not set","mode":1,
            "modeKnown":false,"canPhoto":true,"canRecord":true,
            "isRecording":false,"isTakingPhoto":false}""")!!

        assertFalse(unknown.isVideoMode)
    }

    @Test
    fun `no camera present is no controls`() {
        assertEquals(1, camera("""{"present":true,"labels":["Sony","Thermal"],"selected":1}""")?.selected)
        assertNull(camera("""{"present":true,"labels":[],"selected":null}""")?.selected)
        assertNull(cameraReading(null))
        assertNull(cameraReading(JSONObject("""{"present":false}""")))
    }
}

class CameraModeChangeTest {
    private fun view(extra: String) = JSONObject(
        """{"present":true,"hasModes":true,"modeText":"Photo",$extra}""",
    )

    @Test
    fun `a camera between operations may change mode`() {
        assertTrue(cameraReading(view(""""canChangeMode":true"""))!!.canChangeMode)
    }

    @Test
    fun `a camera mid-operation may not change mode even though it has modes`() {
        val reading = cameraReading(view(""""canChangeMode":false"""))!!
        assertTrue(reading.hasModes)
        assertFalse(reading.canChangeMode)
    }

    @Test
    fun `a view that never mentions canChangeMode does not invent permission`() {
        assertFalse(cameraReading(view(""""mode":1"""))!!.canChangeMode)
    }
}

class CameraTimelapseTest {

    private fun view(
        photoMode: String = "\"timelapse\"",
        canStopPhoto: Boolean = false,
        lapseSeconds: String = "5.0",
        lapseCount: String = "10",
        lapseUnlimited: Boolean = false,
        mode: Int = CAM_MODE_PHOTO,
    ) = JSONObject(
        """{"kind":"object","class":"Camera","present":true,"hasModes":true,"canChangeMode":true,
           "modeText":"Photo","isRecording":false,"canPhoto":true,"canRecord":true,
           "isTakingPhoto":false,"mode":$mode,"modeKnown":true,"photoMode":$photoMode,
           "canStopPhoto":$canStopPhoto,"lapseSeconds":$lapseSeconds,"lapseCount":$lapseCount,
           "lapseUnlimited":$lapseUnlimited}""",
    )

    @Test
    fun `a single-shot camera plans nothing and keeps its old button`() {
        val single = cameraReading(view(photoMode = "\"single\"", lapseSeconds = "null", lapseCount = "null"))!!

        assertNull(lapsePlan(single))
        assertEquals("Take Photo", shutterFor(single)!!.label)
        assertEquals(CAMERA_PHOTO, shutterFor(single)!!.action)
    }

    @Test
    fun `a shutter that starts ten shots does not say Take Photo`() {
        val lapsing = cameraReading(view())!!

        assertEquals("Start lapse", shutterFor(lapsing)!!.label)
        assertEquals("every 5 s, 10 shots", lapsePlan(lapsing))
    }

    @Test
    fun `an unlimited lapse says it will not stop on its own`() {
        val forever = cameraReading(view(lapseCount = "0", lapseUnlimited = true))!!

        assertEquals("every 5 s, until stopped", lapsePlan(forever))
    }

    @Test
    fun `a running interval capture offers the only control that ends it`() {
        val running = cameraReading(view(canStopPhoto = true, lapseUnlimited = true, lapseCount = "0"))!!
        val shutter = shutterFor(running)!!

        assertEquals("Stop lapse", shutter.label)
        assertEquals(CAMERA_STOP_PHOTO, shutter.action)
        assertEquals(true, shutter.enabled)
    }

    @Test
    fun `stopping outranks recording, because a lapse runs in photo mode and cannot wait`() {
        val muddled = cameraReading(view(canStopPhoto = true, mode = CAM_MODE_VIDEO))!!

        assertEquals(CAMERA_STOP_PHOTO, shutterFor(muddled)!!.action)
    }

    @Test
    fun `a whole-second interval is not written with a decimal`() {
        assertEquals("every 5 s, 10 shots", lapsePlan(cameraReading(view())!!))
        assertEquals("every 2.5 s, 10 shots", lapsePlan(cameraReading(view(lapseSeconds = "2.5"))!!))
    }
}

class CameraDetailsTest {

    private fun view(
        reportsStorage: Boolean = true,
        storageText: String = "\"3.2 GB\"",
        shotsText: String = "\"00042\"",
        batteryText: String = "\"87%\"",
        labels: String = """["SimCam","Thermal"]""",
    ) = JSONObject(
        """{"kind":"object","class":"CameraControl","present":true,"title":"SimCam",
           "labels":$labels,"stateText":"Idle","reportsStorage":$reportsStorage,
           "storageText":$storageText,"shotsText":$shotsText,"batteryText":$batteryText,
           "mode":0,"modeKnown":true,"canPhoto":true}""",
    )

    @Test
    fun `the sheet lists what the camera actually reports`() {
        val details = cameraDetails(cameraReading(view())!!)

        assertEquals(
            listOf("State" to "Idle", "Storage" to "3.2 GB", "Photos" to "00042", "Battery" to "87%"),
            details,
        )
    }

    @Test
    fun `a camera that does not report storage gets no storage row rather than a blank one`() {
        val quiet = cameraDetails(cameraReading(view(reportsStorage = false))!!)

        assertEquals(listOf("State", "Photos", "Battery"), quiet.map { it.first })
    }

    @Test
    fun `a camera with no battery reading drops that row too`() {
        val flat = cameraDetails(cameraReading(view(batteryText = "\"\""))!!)

        assertEquals(listOf("State", "Storage", "Photos"), flat.map { it.first })
    }

    @Test
    fun `the labels come from the view, not from a second read of the manager`() {
        assertEquals(listOf("SimCam", "Thermal"), cameraReading(view())!!.labels)
        assertEquals(emptyList<String>(), cameraReading(view(labels = "[]"))!!.labels)
    }
}

class CameraZoomTest {

    private fun camera(hasZoom: Boolean = true, level: Double = 50.0) = cameraReading(
        JSONObject(
            """{"kind":"object","class":"CameraControl","present":true,"hasModes":true,
               "canChangeMode":true,"modeText":"Photo","isRecording":false,"canPhoto":true,
               "canRecord":true,"isTakingPhoto":false,"mode":0,"modeKnown":true,
               "hasZoom":$hasZoom,"zoomLevel":$level}""",
        ),
    )!!

    @Test
    fun `a camera without zoom offers no control at all`() {
        assertNull(zoomText(camera(hasZoom = false)))
        assertNull(zoomStep(camera(hasZoom = false), 10.0))
    }

    @Test
    fun `a step moves by the tick and states where it is`() {
        assertEquals("Zoom 50%", zoomText(camera()))
        assertEquals(60.0, zoomStep(camera(), 10.0)!!, 1e-9)
        assertEquals(40.0, zoomStep(camera(), -10.0)!!, 1e-9)
    }

    @Test
    fun `a step past either end lands on the end rather than beyond it`() {
        assertEquals(100.0, zoomStep(camera(level = 95.0), 10.0)!!, 1e-9)
        assertEquals(0.0, zoomStep(camera(level = 5.0), -10.0)!!, 1e-9)
    }

    @Test
    fun `a camera already at an end cannot step that way, so the control goes dead`() {
        assertNull(zoomStep(camera(level = 100.0), 10.0))
        assertNull(zoomStep(camera(level = 0.0), -10.0))
    }
}

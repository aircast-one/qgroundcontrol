package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CameraControlTest {
    @Test
    fun `an undefined mode offers no shutter`() {
        assertNull(
            shutterFor(CAM_MODE_UNDEFINED, capturesPhotos = true, capturesVideo = true,
                videoStatus = VIDEO_CAPTURE_STOPPED, photoStatus = PHOTO_CAPTURE_IDLE),
        )
        assertNull(cameraModeLabel(CAM_MODE_UNDEFINED))
    }

    @Test
    fun `a camera that cannot do the current mode offers no shutter`() {
        assertNull(
            shutterFor(CAM_MODE_VIDEO, capturesPhotos = true, capturesVideo = false,
                videoStatus = VIDEO_CAPTURE_STOPPED, photoStatus = PHOTO_CAPTURE_IDLE),
        )
        assertNull(
            shutterFor(CAM_MODE_PHOTO, capturesPhotos = false, capturesVideo = true,
                videoStatus = VIDEO_CAPTURE_STOPPED, photoStatus = PHOTO_CAPTURE_IDLE),
        )
    }

    @Test
    fun `video mode reads Record when stopped and Stop while running`() {
        val stopped = shutterFor(CAM_MODE_VIDEO, true, true, VIDEO_CAPTURE_STOPPED, PHOTO_CAPTURE_IDLE)
        val running = shutterFor(CAM_MODE_VIDEO, true, true, VIDEO_CAPTURE_RUNNING, PHOTO_CAPTURE_IDLE)

        assertEquals(CameraShutter("Record", recording = false, enabled = true), stopped)
        assertEquals(CameraShutter("Stop", recording = true, enabled = true), running)
    }

    @Test
    fun `a photo already being taken disables the shutter rather than queueing another`() {
        val busy = shutterFor(CAM_MODE_PHOTO, true, true, VIDEO_CAPTURE_STOPPED, PHOTO_CAPTURE_IN_PROGRESS)

        assertEquals(CameraShutter("Photo", recording = false, enabled = false), busy)
    }

    @Test
    fun `each defined mode is named`() {
        assertEquals("Photo", cameraModeLabel(CAM_MODE_PHOTO))
        assertEquals("Video", cameraModeLabel(CAM_MODE_VIDEO))
    }
}

package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class FirmwareScreenTest {
    @Test
    fun theCoresFirmwareViewsReadAsTheScreenShowsThem() {
        val ports = JSONObject("""{"kind":"object","class":"FirmwarePorts","ports":[{"port":"/dev/bus/usb/001/002","description":"PX4 BL FMU v5.x","bootloader":true},{"port":"","description":"x","bootloader":false}]}""")
        assertEquals(listOf(FirmwarePort("/dev/bus/usb/001/002", "PX4 BL FMU v5.x", true)), firmwarePorts(ports))
        val job = JSONObject("""{"kind":"object","class":"FirmwareUpgrade","phase":"programming","busy":true,"progress":0.25,"messages":["Erasing previous program...","Erase complete"],"error":null}""")
        assertEquals(FirmwareJob("programming", true, 0.25f, listOf("Erasing previous program...", "Erase complete"), ""), firmwareJob(job))
        assertNull("a refusal that is not the firmware view reads as nothing to show", firmwareJob(JSONObject("""{"kind":"null"}""")))
        assertEquals("Waiting for the bootloader", firmwarePhaseText("connecting"))
        assertEquals("", firmwarePhaseText("idle"))
    }

    @Test
    fun onlyImagesTheBootloaderTakesAreAccepted() {
        assertTrue(firmwareFileAccepted("px4_fmu-v5_default.px4"))
        assertTrue(firmwareFileAccepted("arducopter.APJ"))
        assertTrue(firmwareFileAccepted("fw.bin"))
        assertFalse(firmwareFileAccepted("notes.txt"))
    }
}

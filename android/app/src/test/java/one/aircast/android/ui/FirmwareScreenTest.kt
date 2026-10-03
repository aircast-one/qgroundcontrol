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
        val job = JSONObject("""{"kind":"object","class":"FirmwareUpgrade","phase":"programming","busy":true,"cancellable":false,"progress":0.25,"messages":["Erasing previous program...","Erase complete"],"error":null}""")
        assertEquals(FirmwareJob("programming", true, false, 0.25f, listOf("Erasing previous program...", "Erase complete"), ""), firmwareJob(job))
        assertNull("a refusal that is not the firmware view reads as nothing to show", firmwareJob(JSONObject("""{"kind":"null"}""")))
        assertEquals("Waiting for the bootloader", firmwarePhaseText("connecting"))
        assertEquals("", firmwarePhaseText("idle"))
        val choosing = JSONObject("""{"class":"FirmwareUpgrade","phase":"choosing","busy":true,"cancellable":true,"choices":[{"name":"CUAVv5 - 4.6.2","url":"https://f/c.apj"}]}""")
        assertEquals(listOf("CUAVv5 - 4.6.2" to "https://f/c.apj"), firmwareJob(choosing)!!.choices)
        assertEquals("Choose board type", firmwarePhaseText("choosing"))
    }

    @Test
    fun aReleaseIsSentAsTheCoresTokenAndAFileAsItsPath() {
        assertEquals("ardupilot:heli:dev", firmwareChoice("ardupilot:heli:dev", null))
        assertEquals("/cache/firmware-fw.px4", firmwareChoice(FIRMWARE_FROM_FILE, "/cache/firmware-fw.px4"))
        assertNull(firmwareChoice(FIRMWARE_FROM_FILE, null))
        assertTrue(FIRMWARE_SOURCES.any { it.first == "ardupilot:plane:stable" && it.second == "ArduPilot Plane, stable" })
    }

    @Test
    fun onlyImagesTheBootloaderTakesAreAccepted() {
        assertTrue(firmwareFileAccepted("px4_fmu-v5_default.px4"))
        assertTrue(firmwareFileAccepted("arducopter.APJ"))
        assertTrue(firmwareFileAccepted("fw.bin"))
        assertFalse(firmwareFileAccepted("notes.txt"))
    }

    @Test
    fun `beta and developer builds carry QGC's warnings, like FirmwareUpgrade`() {
        assertEquals(BETA_WARNING, firmwareWarning("px4:beta"))
        assertEquals(DEV_WARNING, firmwareWarning("ardupilot:copter:dev"))
        assertEquals(null, firmwareWarning("px4:stable"))
    }

    @Test
    fun `without advanced settings only the standard builds are offered, as FirmwareUpgrade shows`() {
        val plain = firmwareSources(advanced = false).map { it.first }
        assertTrue(plain.all { it.endsWith(":stable") })
        assertTrue("px4:stable" in plain)
        assertTrue(FIRMWARE_FROM_FILE !in plain)
        assertTrue(FIRMWARE_FROM_FILE in firmwareSources(advanced = true).map { it.first })
        assertTrue("px4:dev" in firmwareSources(advanced = true).map { it.first })
        assertEquals(DEV_WARNING, firmwareWarning("px4:dev"))
        assertEquals("ardupilot:plane:stable", sourceAfterAdvanced("ardupilot:plane:dev", advanced = false))
        assertEquals(DEFAULT_FIRMWARE_SOURCE, sourceAfterAdvanced(FIRMWARE_FROM_FILE, advanced = false))
        assertEquals("px4:beta", sourceAfterAdvanced("px4:beta", advanced = true))
    }

    @Test
    fun `the bootloader button needs advanced settings and an ArduPilot vehicle`() {
        assertTrue(bootloaderOffered(advanced = true, apmVehicle = true))
        assertFalse(bootloaderOffered(advanced = false, apmVehicle = true))
        assertFalse(bootloaderOffered(advanced = true, apmVehicle = false))
    }

    @Test
    fun `the last stack and ArduPilot vehicle are remembered like FirmwareUpgrade qml`() {
        org.junit.Assert.assertEquals("px4:stable", rememberedSource(12, 2))
        org.junit.Assert.assertEquals("ardupilot:plane:stable", rememberedSource(3, 2))
        org.junit.Assert.assertEquals("ardupilot:copter:stable", rememberedSource(3, null))
        org.junit.Assert.assertEquals(listOf("defaultFirmwareType" to 3, "apmVehicleType" to 4), sourceSettings("ardupilot:sub:beta"))
        org.junit.Assert.assertEquals(listOf("defaultFirmwareType" to 12), sourceSettings("px4:dev"))
        org.junit.Assert.assertEquals(emptyList<Pair<String, Int>>(), sourceSettings("sik:stable"))
    }

    @Test
    fun `flashing names the board like FirmwareUpgrade qml`() {
        org.junit.Assert.assertEquals("Flashing - Pixhawk 6C", flashingLabel(listOf(FirmwarePort("/dev/bus/usb/1", "Pixhawk 6C", true)), "/dev/bus/usb/1"))
        org.junit.Assert.assertEquals("Flashing - /dev/x", flashingLabel(emptyList(), "/dev/x"))
    }

    @Test
    fun `a recognised selection is kept, else the first Pixhawk, then a SiK radio, like _preselectIndex`() {
        val radio = FirmwarePort("r", "SiK", false, "SiK Radio")
        val fmu = FirmwarePort("p", "Pixhawk 6C", false, "Pixhawk")
        val other = FirmwarePort("o", "FTDI", false)
        org.junit.Assert.assertEquals("p", preselectedPort(listOf(other, radio, fmu), ""))
        org.junit.Assert.assertEquals("r", preselectedPort(listOf(other, radio), "o"))
        org.junit.Assert.assertEquals("r", preselectedPort(listOf(radio, fmu), "r"))
        org.junit.Assert.assertEquals(null, preselectedPort(listOf(other), ""))
    }
}

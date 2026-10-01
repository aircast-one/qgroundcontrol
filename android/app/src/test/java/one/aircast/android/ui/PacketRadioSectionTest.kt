package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PacketRadioSectionTest {
    @Test
    fun `link quality reads like packet radio settings qml`() {
        val status = packetRadioStatus(JSONObject("""{"class":"PacketRadio","statusText":"Receiving on ALFA","linkActive":true,"haveSignal":true,
            "antennaRssiRaw":[60,70],"antennaSnr":[20,25],"linkScore":1800,"packetLoss":2,"videoPackets":1234,"adapters":["ALFA [1]"]}"""))!!
        assertEquals("60  |  70", status.signal)
        assertEquals("20  |  25 dB", status.noise)
        assertEquals("1800", status.linkScore)
        assertEquals(listOf("ALFA [1]"), status.adapters)
        val quiet = packetRadioStatus(JSONObject("""{"class":"PacketRadio","statusText":"Listening","linkActive":true,"haveSignal":false}"""))!!
        assertEquals("waiting" to "waiting", quiet.signal to quiet.linkScore)
        assertNull(packetRadioStatus(JSONObject("""{"kind":"null"}""")))
    }
}

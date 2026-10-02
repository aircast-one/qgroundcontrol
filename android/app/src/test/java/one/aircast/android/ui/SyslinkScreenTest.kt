package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class SyslinkScreenTest {

    @Test
    fun `the radio settings read as SyslinkComponent shows them`() {
        val radio = syslink(JSONObject("""{"available":true,"channel":80,"address":"e7e7e7e7e7","rate":2,"rates":["750Kb/s","1Mb/s","2Mb/s"],"channelHint":"Channel can be between 0 and 125","addressHint":"Address in hex. Default is E7E7E7E7E7."}"""))!!
        assertEquals(80, radio.channel)
        assertEquals("2Mb/s", radio.rates[radio.rate])
        assertNull(syslink(JSONObject("""{"available":false}""")))
    }

    @Test
    fun `the address field takes hex only, as its RegExpValidator does`() {
        assertTrue(hexAddress("E7e7"))
        assertTrue(hexAddress(""))
        assertFalse(hexAddress("E7G"))
    }
}

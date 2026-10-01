package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class AircastCloudTest {
    @Test
    fun `fields validate like aircast cloud settings qml`() {
        assertTrue(cloudApiBaseValid("https://api.aircast.one"))
        assertTrue(cloudApiBaseValid(" http://10.0.0.2:8080 "))
        assertFalse(cloudApiBaseValid("api.aircast.one"))
        assertFalse(cloudApiBaseValid("https:///x"))
        assertFalse(cloudDeviceValid("  "))
    }

    @Test
    fun `the account line reads signed in, the status, or not signed in`() {
        val signedIn = accountState(JSONObject("""{"kind":"object","signedIn":true,"signingIn":false,"status":"","userCode":"","verificationUrl":""}"""))!!
        assertEquals("Signed in", accountLine(signedIn))
        assertEquals("Not signed in", accountLine(signedIn.copy(signedIn = false)))
        assertEquals("Approve code ABC in the browser to sign in.", accountLine(signedIn.copy(signedIn = false, status = "Approve code ABC in the browser to sign in.")))
        assertNull(accountState(JSONObject("""{"kind":"null"}""")))
    }
}

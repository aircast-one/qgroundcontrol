package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class RemoteSupportScreenTest {

    private fun served(valid: Boolean, error: String) = JSONObject(
        """{"kind":"object","class":"SupportHost","valid":$valid,"error":"$error"}""",
    )

    @Test
    fun `the head reports the core's sentence, not one of its own`() {
        val refused = supportHostVerdict(
            served(false, "The part after the colon has to be a port number between 1 and 65535."),
        )!!

        assertFalse(refused.valid)
        assertEquals(
            "three different refusals each get their own words now; the head used to say " +
                "'enter the address your support engineer gave you' to all of them",
            "The part after the colon has to be a port number between 1 and 65535.",
            refused.error,
        )
    }

    @Test
    fun `an accepted host carries no complaint`() {
        val ok = supportHostVerdict(served(true, ""))!!

        assertTrue(ok.valid)
        assertEquals("", ok.error)
    }

    @Test
    fun `a view of another shape is not a verdict`() {
        assertNull(supportHostVerdict(null))
        assertNull(
            "answering from whatever happens to be at the path would let an unrelated " +
                "object enable a button that starts forwarding",
            supportHostVerdict(JSONObject("""{"kind":"object","class":"Links"}""")),
        )
    }

    @Test
    fun `the query names the host it is asking about`() {
        assertEquals("view.supportHost(10.0.0.4:14550)", supportHostPath("10.0.0.4:14550"))
    }

    @Test
    fun `the core answers the comma now, so the head does not`() {
        assertNull(
            "view.supportHost sees a typed comma as a second argument - which cannot arise " +
                "any other way - and refuses it. The head asked because it thought the " +
                "truncation hid the evidence; the truncation is the evidence",
            supportHostCannotBeAsked("10.0.0.4,14550"),
        )
        assertNull(supportHostCannotBeAsked("two words:14550"))
    }

    @Test
    fun `surrounding space stays refused here, because the core trims and QGC does not`() {
        assertEquals(
            "links.rs trims the typed value before judging it, so '  host:14550' is answered " +
                "valid - but LinkManager passes the stored value to addHost untrimmed, and " +
                "QHostInfo::fromName cannot resolve a name with a leading space, so addHost " +
                "returns at UDPLink.cc:166 without adding a client. Valid, and forwards nowhere",
            "Remove the space before or after the address.",
            supportHostCannotBeAsked("  leading:14550"),
        )
        assertEquals(
            "Remove the space before or after the address.",
            supportHostCannotBeAsked("host:14550 "),
        )
        assertNull(supportHostCannotBeAsked("host:14550"))
        assertNull("a blank field is the empty state, not a complaint", supportHostCannotBeAsked(""))
    }
}
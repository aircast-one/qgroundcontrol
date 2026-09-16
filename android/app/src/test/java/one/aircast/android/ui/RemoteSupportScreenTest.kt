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
    fun `a comma is refused here, because the path cannot carry it`() {
        assertEquals(
            "view arguments split on a comma, so a host containing one would arrive at the " +
                "core truncated and could be answered valid on the half that survived",
            "An address cannot contain a comma.",
            supportHostCannotBeAsked("10.0.0.4,14550"),
        )
        assertNull(supportHostCannotBeAsked("10.0.0.4:14550"))
    }

    @Test
    fun `a spaced address is refused here, because the core still accepts it`() {
        assertEquals(
            "measured on device: view.supportHost answers valid for 'two words:14550' and " +
                "every other whitespace form, while UDPLink.cc:166 returns without adding a " +
                "client when the address will not resolve - so adopting the served verdict " +
                "alone would put back the defect 691db8606 removed. Drop this when the core " +
                "refuses whitespace",
            "An address cannot contain a space.",
            supportHostCannotBeAsked("two words:14550"),
        )
        assertEquals("An address cannot contain a space.", supportHostCannotBeAsked("has space.org"))
        assertNull("a blank field is the empty state, not a complaint", supportHostCannotBeAsked(""))
    }
}

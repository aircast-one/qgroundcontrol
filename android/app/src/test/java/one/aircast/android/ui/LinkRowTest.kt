package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

private const val TWO_LINKS = """
{"available": true, "class": "Links", "configured": [
  {"index": 0, "name": "Packet radio", "type": "udp", "typeLabel": "UDP",
   "statusLine": "Connected · UDP port 14550", "connected": true, "heardVehicle": true,
   "goneQuiet": false, "dynamic": false, "lastError": "", "editing": "portOnly",
   "summary": "UDP port 14550", "displaySummary": "UDP port 14550", "path": "links.linkConfigurations.0"},
  {"index": 1, "name": "UDP 14551", "type": "udp", "typeLabel": "UDP",
   "statusLine": "Not hearing the vehicle · UDP port 14551", "connected": true, "heardVehicle": true,
   "goneQuiet": true, "dynamic": false, "lastError": "", "editing": "portOnly",
   "summary": "UDP port 14551", "displaySummary": "UDP port 14551", "path": "links.linkConfigurations.1"}
]}
"""

class LinkRowTest {
    @Test
    fun `a link that has gone quiet is carried through, not collapsed into heard`() {
        val rows = linkRows(JSONObject(TWO_LINKS))
        val quiet = rows.single { it.name == "UDP 14551" }
        assertTrue("heardVehicle stays true once a packet has ever arrived", quiet.heard)
        assertTrue("so the row needs the core's separate answer to tell the operator", quiet.goneQuiet)
        assertEquals("Not hearing the vehicle · UDP port 14551", quiet.statusLine)
    }

    @Test
    fun `the link still carrying the vehicle is not marked quiet`() {
        val live = linkRows(JSONObject(TWO_LINKS)).single { it.name == "Packet radio" }
        assertTrue(live.heard)
        assertFalse(live.goneQuiet)
    }
}

class RemedyTest {
    @Test
    fun `an address that nothing answers says retrying will not help`() {
        assertEquals(
            "Nothing is listening at that address. Retrying will not help until it is changed.",
            remedyText(REMEDY_EDIT_ADDRESS),
        )
    }

    @Test
    fun `a retryable error adds no advice, because Connect already says it`() {
        assertNull(remedyText("retry"))
        assertNull(remedyText(""))
        assertNull(remedyText("somethingTheCoreAddedLater"))
    }
}

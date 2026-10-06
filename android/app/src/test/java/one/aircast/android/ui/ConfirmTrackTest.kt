package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ConfirmTrackTest {

    @Test
    fun `sent stands until the served actions change`() {
        assertTrue(
            "the operator needs to see the command left the handset while the link is still " +
                "carrying it; reverting to buttons straight away reads as nothing happened",
            sentIsStillShowing("Land", "{A}", "{A}"),
        )
        assertFalse(
            "once the vehicle's own flags move, the next state answers the question and the " +
                "notice has nothing left to say",
            sentIsStillShowing("Land", "{A}", "{B}"),
        )
    }

    @Test
    fun `nothing was sent means nothing is shown`() {
        assertFalse(sentIsStillShowing(null, "{A}", "{A}"))
        assertFalse(sentIsStillShowing("Land", null, "{A}"))
    }

    @Test
    fun `a send recorded against no reading does not stick`() {
        assertFalse(
            "a null snapshot at send would match a null live read and hold the notice forever",
            sentIsStillShowing("Land", null, null),
        )
    }

    @Test
    fun `the notice names the action that was sent`() {
        assertEquals("Sent · Return", sentText("Return"))
        assertEquals("Sent · Emergency Stop", sentText("Emergency Stop"))
    }

    @org.junit.Test
    fun `the land confirm reads the height it lands from, and nothing when the vehicle has not said`() {
        org.junit.Assert.assertEquals("42.0" to "m", landFrom(org.json.JSONObject("""{"items":[{"missing":false,"value":"42.0","units":"m"}]}""")))
        org.junit.Assert.assertNull(landFrom(org.json.JSONObject("""{"items":[{"missing":true,"value":"\u2014","units":""}]}""")))
        org.junit.Assert.assertNull(landFrom(null))
    }

    @Test
    fun `starting a mission names the mission the drone will fly`() {
        val summary = org.json.JSONObject("""{"rows":[{"id":"distance","label":"Distance","value":"1.40 km"},{"id":"time","label":"Time","value":"6 min"}]}""")
        val uploaded = org.json.JSONObject("""{"hasMissionItems":true,"dirty":false,"file":"ridge.plan","status":"Uploaded \u00b7 12 items"}""")
        assertEquals(MissionIdentity("ridge", "Uploaded \u00b7 12 items \u00b7 1.40 km \u00b7 6 min", null), missionIdentity(uploaded, summary))
        val edited = org.json.JSONObject("""{"hasMissionItems":true,"dirty":true,"file":"ridge.plan","status":"Edited \u00b7 13 items"}""")
        assertEquals(MISSION_ON_DRONE, missionIdentity(edited, summary).title)
        assertTrue(missionIdentity(edited, summary).warning!!.startsWith("ridge has changes"))
        assertEquals(MISSION_ON_DRONE, missionIdentity(org.json.JSONObject("""{"hasMissionItems":false}"""), summary).title)
        assertEquals("Hold to start mission", holdLabel("Start mission"))
        assertEquals("Hold to land", holdLabel("Land"))
        assertEquals("Hold to confirm", holdLabel(""))
        assertFalse("land" in MISSION_ACTIONS)
    }
}

package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class MissionCompleteDialogTest {
    @Test
    fun `an open notice carries what the dialog offers`() {
        val notice = missionComplete(
            JSONObject("""{"open":true,"id":3,"imagesTaken":12,"resumeFromWaypoint":4,"batteryWarning":true}"""),
        )!!
        assertEquals(3L, notice.id)
        assertEquals(4, notice.resumeFromWaypoint)
        assertTrue(notice.batteryWarning)
        assertEquals("12 Images Taken", imagesTakenText(notice.imagesTaken))
    }

    @Test
    fun `a closed notice or no photos shows nothing`() {
        assertNull(missionComplete(JSONObject("""{"open":false,"id":3}""")))
        assertNull(missionComplete(JSONObject("""{"open":true,"id":1,"resumeFromWaypoint":null}"""))!!.resumeFromWaypoint)
        assertNull(imagesTakenText(0))
    }

    @Test
    fun `an interrupted mission says so and names where it resumes`() {
        val notice = missionComplete(
            JSONObject("""{"open":true,"id":2,"imagesTaken":0,"resumeFromWaypoint":5,"batteryWarning":true,"interrupted":true,"title":"Mission interrupted","detail":"2 drops are still to come.","resumeLabel":"Resume from waypoint 4"}"""),
        )!!
        assertEquals(listOf("Mission interrupted", "Resume from waypoint 4"), listOf(notice.title, notice.resumeLabel))
        assertTrue(notice.interrupted)
        assertEquals("Flight plan complete", missionComplete(JSONObject("""{"open":true,"id":3,"resumeFromWaypoint":null}"""))!!.title)
    }
}

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
}

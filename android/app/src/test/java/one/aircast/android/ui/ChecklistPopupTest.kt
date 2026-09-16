package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChecklistPopupTest {

    private fun served(vararg checks: String, blocked: Int = 0) = preflight(
        JSONObject(
            """{"kind":"object","class":"Preflight","total":${checks.size},
                "blocked":[${(0 until blocked).joinToString(",") { "\"stop $it\"" }}],
                "groups":[{"name":"Before flight","checks":[${checks.joinToString(",")}]}]}""",
        ),
    )

    private fun manual(name: String) = """{"name":"$name","verdict":"manual","text":"$name"}"""
    private fun automatic(name: String) = """{"name":"$name","verdict":"pass","text":"$name"}"""

    @Test
    fun `an unticked manual check leaves the list incomplete`() {
        assertFalse(checklistIsComplete(served(manual("Props on")), emptySet()))
        assertTrue(checklistIsComplete(served(manual("Props on")), setOf("Props on")))
    }

    @Test
    fun `a blocking check is never complete, however much is ticked`() {
        assertFalse(
            "a blocker stops the flight; ticking the manual items around it does not clear it",
            checklistIsComplete(served(manual("Props on"), blocked = 1), setOf("Props on")),
        )
    }

    @Test
    fun `a list with nothing to tick is already complete`() {
        assertTrue(checklistIsComplete(served(automatic("Battery")), emptySet()))
    }

    @Test
    fun `no checklist at all is not complete`() {
        assertFalse(
            "absent is not passed - with no vehicle there is nothing to have checked",
            checklistIsComplete(null, emptySet()),
        )
    }

    @Test
    fun `the popup is due only when every flag Qt reads is set`() {
        assertTrue(checklistPopupIsDue(true, useChecklist = true, enforceChecklist = true, complete = false))
        assertFalse(
            "no vehicle, nothing to check",
            checklistPopupIsDue(false, useChecklist = true, enforceChecklist = true, complete = false),
        )
        assertFalse(
            "the operator turned the checklist off",
            checklistPopupIsDue(true, useChecklist = false, enforceChecklist = true, complete = false),
        )
        assertFalse(
            "enforcement off means the list is available but not pushed",
            checklistPopupIsDue(true, useChecklist = true, enforceChecklist = false, complete = false),
        )
        assertFalse(
            "already done, so opening it would interrupt for nothing",
            checklistPopupIsDue(true, useChecklist = true, enforceChecklist = true, complete = true),
        )
    }
}

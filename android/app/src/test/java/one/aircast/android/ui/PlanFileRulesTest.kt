package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class PlanFileRulesTest {
    @Test
    fun `a reason the core declines to give does not reach the screen as the word null`() {
        assertEquals(
            "optString renders a JSON null as the four letters n-u-l-l on Android, which is not " +
                "blank, so ifBlank would not fire and the operator would be told 'null'. The core " +
                "is moving these sentences to null-when-absent, which makes that reachable",
            "The plan could not be checked for saving.",
            saveBlockedReason(JSONObject("""{"readiness":{"ready":false,"reason":null}}""")),
        )
        assertEquals(
            "The plan could not be checked for saving.",
            saveBlockedReason(JSONObject("""{"readiness":{"ready":false,"reason":""}}""")),
        )
        assertEquals(
            "This plan has no items.",
            saveBlockedReason(JSONObject("""{"readiness":{"ready":false,"reason":"This plan has no items."}}""")),
        )
    }
}

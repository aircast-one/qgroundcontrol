package one.aircast.android.ui

import org.json.JSONObject

internal const val PREFLIGHT = "view.preflight"

internal data class PreflightCheck(
    val name: String,
    val prompt: String,
    val verdict: String,
    val reason: String,
    val blocked: Boolean,
)

internal data class PreflightGroup(val name: String, val checks: List<PreflightCheck>)

internal data class Preflight(
    val airframe: String,
    val total: Int,
    val blocked: List<String>,
    val groups: List<PreflightGroup>,
)

internal fun preflight(view: JSONObject?): Preflight? {
    val groups = view?.optJSONArray("groups") ?: return null
    val blocked = view.optJSONArray("blocked")
    return Preflight(
        airframe = view.optString("airframe"),
        total = view.optInt("total"),
        blocked = (0 until (blocked?.length() ?: 0)).map { blocked!!.optString(it) },
        groups = (0 until groups.length()).mapNotNull { index ->
            groups.optJSONObject(index)?.let { group ->
                val checks = group.optJSONArray("checks")
                PreflightGroup(
                    name = group.optString("name"),
                    checks = (0 until (checks?.length() ?: 0)).mapNotNull { check ->
                        checks!!.optJSONObject(check)?.let {
                            PreflightCheck(
                                name = it.optString("name"),
                                prompt = it.optString("prompt"),
                                verdict = it.optString("verdict"),
                                reason = it.optString("reason"),
                                blocked = it.optBoolean("blocked"),
                            )
                        }
                    },
                )
            }
        },
    )
}

internal fun preflightSummary(preflight: Preflight?, ticked: Set<String>): String {
    if (preflight == null) return "Connect a vehicle to run its preflight checks."
    if (preflight.blocked.isNotEmpty()) {
        return "${preflight.blocked.size} of ${preflight.total} will stop the flight."
    }
    val checks = preflight.groups.flatMap { it.checks }
    val manual = checks.filter(::checkNeedsTicking).map { it.name }.toSet()
    val outstanding = (manual - ticked).size
    val warnings = checks.count { it.verdict == "overridable" }
    return when {
        outstanding > 0 -> "$outstanding of ${manual.size} left to check."
        warnings > 0 ->
            "All ${preflight.total} checks done · $warnings warning${if (warnings == 1) "" else "s"}."
        else -> "All ${preflight.total} checks done."
    }
}

internal fun checkNeedsTicking(check: PreflightCheck): Boolean = check.verdict == "manual"

internal fun checkStatusText(check: PreflightCheck, ticked: Boolean): String = when (check.verdict) {
    "passing" -> "Passing"
    "failing" -> check.reason.ifBlank { "Failing" }
    "overridable" -> check.reason.ifBlank { "Needs attention" }
    else -> if (ticked) "Checked" else "Check it"
}

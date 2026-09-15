package one.aircast.android.ui

import org.json.JSONObject
import one.aircast.mapspike.optText

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
        airframe = view.optText("airframe"),
        total = view.optInt("total"),
        blocked = (0 until (blocked?.length() ?: 0)).map { blocked!!.optText(it) },
        groups = (0 until groups.length()).mapNotNull { index ->
            groups.optJSONObject(index)?.let { group ->
                val checks = group.optJSONArray("checks")
                PreflightGroup(
                    name = group.optText("name"),
                    checks = (0 until (checks?.length() ?: 0)).mapNotNull { check ->
                        checks!!.optJSONObject(check)?.let {
                            PreflightCheck(
                                name = it.optText("name"),
                                prompt = it.optText("prompt"),
                                verdict = it.optText("verdict"),
                                reason = it.optText("reason"),
                                blocked = it.optBoolean("blocked"),
                            )
                        }
                    },
                )
            }
        },
    )
}

internal fun checklistOffered(armed: Boolean): String? = when {
    armed -> "The checks are for before the flight"
    else -> null
}

internal fun preflightOffered(view: JSONObject?): Boolean = view?.optBoolean("offered", true) ?: true

internal fun preflightSummary(preflight: Preflight?, ticked: Set<String>): String {
    if (preflight == null) return "Connect a vehicle to run its preflight checks."
    val checks = preflight.groups.flatMap { it.checks }
    val manual = checks.filter(::checkNeedsTicking).map { it.name }.toSet()
    val outstanding = (manual - ticked).size
    val warnings = checks.count { it.verdict == "overridable" }
    if (preflight.blocked.isNotEmpty()) {
        val left = if (outstanding > 0) " · $outstanding left to check" else ""
        return "${preflight.blocked.size} of ${preflight.total} will stop the flight$left."
    }
    return when {
        outstanding > 0 -> "$outstanding of ${manual.size} left to check."
        warnings > 0 ->
            "All ${preflight.total} checks done · $warnings warning${if (warnings == 1) "" else "s"}."
        else -> "All ${preflight.total} checks done."
    }
}

internal fun checkNeedsTicking(check: PreflightCheck): Boolean = check.verdict == "manual"

internal enum class CheckMark { TICKABLE, PASSED, ATTENTION }

internal fun checkMark(check: PreflightCheck): CheckMark = when {
    checkNeedsTicking(check) -> CheckMark.TICKABLE
    check.verdict == "passing" -> CheckMark.PASSED
    else -> CheckMark.ATTENTION
}

internal fun checkStatusText(check: PreflightCheck, ticked: Boolean): String = when (check.verdict) {
    "passing" -> "Passing"
    "failing" -> check.reason.ifBlank { "Failing" }
    "overridable" -> check.reason.ifBlank { "Needs attention" }
    else -> if (ticked) "Checked" else "Check it"
}

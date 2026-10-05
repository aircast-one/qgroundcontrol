package one.aircast.android.ui

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcPath
import one.aircast.android.bridge.settingControl

private const val CHECKLIST_CLOSE_DELAY_MS = 1000L

internal class PreflightChecklistState(private val tickedState: MutableState<Set<String>>) {
    var ticked by tickedState
    var shown by mutableStateOf(false)
    var popupShownFor by mutableStateOf<Int?>(null)
    var stateSent by mutableStateOf<Boolean?>(null)

    fun open() {
        shown = true
    }
}

@Composable
internal fun rememberPreflightChecklist(): PreflightChecklistState {
    val ticked = rememberSaveable { mutableStateOf(setOf<String>()) }
    return remember(ticked) { PreflightChecklistState(ticked) }
}

@Composable
internal fun PreflightChecklistReset(checklist: PreflightChecklistState, available: Boolean) {
    LaunchedEffect(available) {
        if (!available) {
            checklist.ticked = emptySet()
            checklist.popupShownFor = null
        }
    }
}

@Composable
internal fun PreflightChecklist(checklist: PreflightChecklistState, deciding: Boolean) {
    val enforceChecklist by qgcBool(settingControl("settings.appSettings.enforceChecklist"))
    val preflightJson by qgcPath(PREFLIGHT)
    val offered = remember(preflightJson) { preflightOffered(preflightJson) }
    val checks = remember(preflightJson) { preflight(preflightJson) }
    val vehiclesJson by qgcPath(one.aircast.map.VEHICLES_VIEW)
    val vehicleId = remember(vehiclesJson) { activeVehicleId(vehiclesJson) }
    LaunchedEffect(vehicleId) {
        checklist.ticked = emptySet()
        checklist.stateSent = null
    }
    val passed = checklistIsComplete(checks, checklist.ticked)
    LaunchedEffect(passed) {
        if (passed && checklist.shown) {
            delay(CHECKLIST_CLOSE_DELAY_MS)
            checklist.shown = false
        }
    }
    LaunchedEffect(passed, vehicleId) {
        if (vehicleId == null || checklist.stateSent == passed) return@LaunchedEffect
        if (checklist.stateSent == null && !passed) {
            checklist.stateSent = false
            return@LaunchedEffect
        }
        checklist.stateSent = passed
        withContext(Dispatchers.Default) { Qgc.set("vehicle.checkListState", checklistStateValue(passed)) }
    }
    LaunchedEffect(vehicleId, deciding) {
        val id = vehicleId ?: return@LaunchedEffect
        if (checklist.popupShownFor == id || deciding) return@LaunchedEffect
        delay(CHECKLIST_POPUP_DELAY_MS)
        val complete = withContext(Dispatchers.Default) {
            checklistIsComplete(preflight(Qgc.get(PREFLIGHT)), checklist.ticked)
        }
        if (checklistPopupIsDue(true, offered, enforceChecklist, complete, deciding)) {
            checklist.popupShownFor = id
            checklist.shown = true
        }
    }
    if (checklist.shown) {
        AlertDialog(
            onDismissRequest = { checklist.shown = false },
            title = { Text("Pre-flight checklist") },
            text = {
                PreflightScreen(
                    modifier = Modifier.fillMaxWidth(),
                    ticked = checklist.ticked,
                    onTicked = { checklist.ticked = it },
                )
            },
            confirmButton = {},
            dismissButton = { TextButton(onClick = { checklist.shown = false }) { Text("Close") } },
        )
    }
}

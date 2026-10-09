package one.aircast.map

import androidx.annotation.DrawableRes
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject

data class WaypointYaw(val degrees: Double?, val units: String, val path: String)

fun waypointYaw(view: JSONObject?): WaypointYaw? =
    view?.optJSONObject("yaw")?.let {
        WaypointYaw(
            degrees = if (it.isNull("value")) null else it.optDouble("value").takeIf { value -> !value.isNaN() },
            units = it.optText("units").ifBlank { "deg" },
            path = it.optText("path"),
        )
    }?.takeIf { it.path.isNotBlank() }

internal enum class ActionKind { Hover, Camera, Gimbal, Mode, Turn }

internal data class WaypointAction(val kind: ActionKind, val title: String, val value: String, @DrawableRes val icon: Int)

internal data class ActionOffer(val title: String, @DrawableRes val icon: Int, val kind: ActionKind, val choice: Int = -1)

private const val DEFAULT_HOVER_SECONDS = 5.0
private const val NO_CAMERA_ACTION = 0
private val HOVER_RANGE = 0.0..600.0
private val PITCH_RANGE = -90.0..30.0
private val HEADING_RANGE = -180.0..180.0
private val INTERVAL_RANGE = 1.0..3600.0
private const val ANGLE_STEP = 5.0

internal fun cameraIcon(label: String): Int = when {
    label.contains("video", ignoreCase = true) && label.contains("stop", ignoreCase = true) -> R.drawable.plan_video_off
    label.contains("video", ignoreCase = true) -> R.drawable.plan_video
    label.contains("stop", ignoreCase = true) -> R.drawable.plan_stop
    else -> R.drawable.plan_photo
}

internal fun degreesText(value: Double): String = "${trimmedNumber(value)}°"

internal fun cameraInterval(extras: CameraExtras?): String = when {
    extras?.intervalTime != null -> "every ${trimmedNumber(extras.intervalTime)} s"
    extras?.intervalDistance != null -> "every ${trimmedNumber(extras.intervalDistance)} ${extras.distanceUnits}"
    else -> ""
}

internal fun startingChoice(labels: List<String>, subject: String): Int? =
    labels.indexOfFirst { it.contains(subject, ignoreCase = true) && !it.contains("stop", ignoreCase = true) }.takeIf { it > NO_CAMERA_ACTION }

internal fun activeActions(hold: WaypointHold?, yaw: WaypointYaw?, choices: CameraChoices?, extras: CameraExtras?): List<WaypointAction> = listOfNotNull(
    hold?.takeIf { it.seconds > 0.0 }?.let { WaypointAction(ActionKind.Hover, "Hover", "${trimmedNumber(it.seconds)} ${it.units}", R.drawable.plan_timer) },
    choices?.takeIf { it.chosen > NO_CAMERA_ACTION }?.let { picked ->
        val label = picked.labels.getOrElse(picked.chosen) { "" }
        WaypointAction(ActionKind.Camera, sentenceCase(label), cameraInterval(extras), cameraIcon(label))
    },
    extras?.takeIf { it.commandsGimbal }?.let { WaypointAction(ActionKind.Gimbal, "Gimbal", "${degreesText(it.pitch)} / ${degreesText(it.yaw)}", R.drawable.plan_gimbal) },
    extras?.takeIf { it.modeSupported && it.commandsMode }?.let { WaypointAction(ActionKind.Mode, "Camera mode", CAMERA_MODES.getOrElse(it.mode) { "" }, R.drawable.plan_tune) },
    yaw?.degrees?.let { WaypointAction(ActionKind.Turn, "Turn aircraft", degreesText(it), R.drawable.plan_turn) },
)

internal fun offeredActions(hold: WaypointHold?, yaw: WaypointYaw?, choices: CameraChoices?, extras: CameraExtras?): List<ActionOffer> =
    listOfNotNull(hold?.takeIf { it.seconds <= 0.0 }?.let { ActionOffer("Hover", R.drawable.plan_timer, ActionKind.Hover) }) +
        choices?.takeIf { it.chosen <= NO_CAMERA_ACTION }?.let { camera ->
            listOfNotNull(
                startingChoice(camera.labels, "photo")?.let { ActionOffer("Photo", R.drawable.plan_photo, ActionKind.Camera, it) },
                startingChoice(camera.labels, "video")?.let { ActionOffer("Video", R.drawable.plan_video, ActionKind.Camera, it) },
            )
        }.orEmpty() +
        listOfNotNull(
            extras?.takeIf { !it.commandsGimbal }?.let { ActionOffer("Tilt camera", R.drawable.plan_gimbal, ActionKind.Gimbal) },
            extras?.takeIf { it.modeSupported && !it.commandsMode }?.let { ActionOffer("Camera mode", R.drawable.plan_tune, ActionKind.Mode) },
            yaw?.takeIf { it.degrees == null }?.let { ActionOffer("Turn aircraft", R.drawable.plan_turn, ActionKind.Turn) },
        )

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun WaypointActions(index: Int, hold: WaypointHold?, yaw: WaypointYaw?) {
    var revision by remember(index) { mutableIntStateOf(0) }
    val camera by produceState<JSONObject?>(null, index, revision) {
        value = withContext(Dispatchers.Default) { ItemCameraBridge.read(index) }
    }
    val choices = cameraChoices(camera)
    val extras = cameraExtras(camera)
    val active = activeActions(hold, yaw, choices, extras)
    val offers = offeredActions(hold, yaw, choices, extras)
    if (active.isEmpty() && offers.isEmpty()) return
    val scope = rememberCoroutineScope()
    var adding by remember { mutableStateOf(false) }
    var editing by remember(index) { mutableStateOf<ActionKind?>(null) }
    val write: (() -> Boolean) -> Unit = { work ->
        scope.launch {
            withContext(Dispatchers.Default) { work() }
            revision += 1
        }
    }
    val setHold: (Double) -> Unit = { seconds -> hold?.let { write { setOk(it.path, settingJson("$seconds")) } } }
    val setYaw: (Double?) -> Unit = { degrees -> yaw?.let { write { setOk(it.path, settingJson(degrees?.toString() ?: "null")) } } }
    val setCamera: (String, Any) -> Unit = { member, value -> write { ItemCameraBridge.set(index, member, value) } }
    val remove: (ActionKind) -> Unit = { kind ->
        when (kind) {
            ActionKind.Hover -> setHold(0.0)
            ActionKind.Camera -> write { ItemCameraBridge.chooseAction(index, NO_CAMERA_ACTION) }
            ActionKind.Gimbal -> setCamera("specifyGimbal", false)
            ActionKind.Mode -> setCamera("specifyCameraMode", false)
            ActionKind.Turn -> setYaw(null)
        }
    }

    Column(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("Actions", style = MaterialTheme.typography.titleSmall, modifier = Modifier.weight(1f))
            if (offers.isNotEmpty()) Box {
                TextButton(onClick = { adding = true }) {
                    Icon(painterResource(R.drawable.plan_add), contentDescription = null, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(4.dp))
                    Text("Add")
                }
                DropdownMenu(expanded = adding, onDismissRequest = { adding = false }) {
                    offers.forEach { offer ->
                        DropdownMenuItem(
                            text = { Text(offer.title) },
                            leadingIcon = { Icon(painterResource(offer.icon), contentDescription = null) },
                            onClick = {
                                adding = false
                                when (offer.kind) {
                                    ActionKind.Hover -> setHold(DEFAULT_HOVER_SECONDS)
                                    ActionKind.Camera -> write { ItemCameraBridge.chooseAction(index, offer.choice) }
                                    ActionKind.Gimbal -> setCamera("specifyGimbal", true)
                                    ActionKind.Mode -> setCamera("specifyCameraMode", true)
                                    ActionKind.Turn -> setYaw(0.0)
                                }
                                editing = offer.kind
                            },
                        )
                    }
                }
            }
        }
        if (active.isEmpty()) {
            Text("Tap Add to take a photo, film, hover or turn here.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            active.forEach { action -> ActionChip(action, onClick = { editing = action.kind }, onRemove = { remove(action.kind) }) }
        }
    }

    editing?.let { kind ->
        AlertDialog(
            onDismissRequest = { editing = null },
            title = { Text(active.firstOrNull { it.kind == kind }?.title ?: offers.firstOrNull { it.kind == kind }?.title.orEmpty()) },
            text = {
                Column {
                    when (kind) {
                        ActionKind.Hover -> hold?.let { SettingStepper("Hover", it.seconds, it.units, 1.0, onSet = setHold, range = HOVER_RANGE, slider = true) }
                        ActionKind.Turn -> SettingStepper("Heading", yaw?.degrees ?: 0.0, "°", ANGLE_STEP, onSet = { setYaw(it) }, range = HEADING_RANGE, slider = true)
                        ActionKind.Gimbal -> extras?.let { camera ->
                            SettingStepper("Pitch", camera.pitch, "°", ANGLE_STEP, onSet = { setCamera("gimbalPitch", it) }, range = camera.pitchRange ?: PITCH_RANGE, slider = true)
                            SettingStepper("Yaw", camera.yaw, "°", ANGLE_STEP, onSet = { setCamera("gimbalYaw", it) }, range = camera.yawRange ?: HEADING_RANGE, slider = true)
                        }
                        ActionKind.Mode -> Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            CAMERA_MODES.forEachIndexed { at, label ->
                                FilterChip(selected = extras?.mode == at, onClick = { setCamera("cameraMode", at) }, label = { Text(label) })
                            }
                        }
                        ActionKind.Camera -> {
                            choices?.labels.orEmpty().withIndex().drop(1).forEach { (at, label) ->
                                Row(
                                    Modifier.fillMaxWidth().heightIn(min = 48.dp).clickable { write { ItemCameraBridge.chooseAction(index, at) } },
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    RadioButton(selected = choices?.chosen == at, onClick = { write { ItemCameraBridge.chooseAction(index, at) } })
                                    Text(sentenceCase(label))
                                }
                            }
                            extras?.intervalTime?.let { seconds ->
                                SettingStepper("Every", seconds, "s", 1.0, onSet = { setCamera("cameraPhotoIntervalTime", it) }, range = INTERVAL_RANGE)
                            }
                            extras?.intervalDistance?.let { distance ->
                                SettingStepper("Every", distance, extras.distanceUnits, 1.0, onSet = { setCamera("cameraPhotoIntervalDistance", it) }, range = INTERVAL_RANGE)
                            }
                        }
                    }
                }
            },
            confirmButton = { TextButton(onClick = { editing = null }) { Text("Done") } },
        )
    }
}

@Composable
private fun ActionChip(action: WaypointAction, onClick: () -> Unit, onRemove: () -> Unit) {
    Surface(
        onClick = onClick,
        shape = RoundedCornerShape(14.dp),
        color = MaterialTheme.colorScheme.secondaryContainer,
        contentColor = MaterialTheme.colorScheme.onSecondaryContainer,
    ) {
        Row(Modifier.padding(start = 12.dp), verticalAlignment = Alignment.CenterVertically) {
            Icon(painterResource(action.icon), contentDescription = null, modifier = Modifier.size(20.dp))
            Column(Modifier.padding(start = 8.dp, top = 6.dp, bottom = 6.dp)) {
                Text(action.title, style = MaterialTheme.typography.labelLarge)
                if (action.value.isNotBlank()) Text(action.value, style = MaterialTheme.typography.labelSmall)
            }
            IconButton(onClick = onRemove, modifier = Modifier.size(40.dp)) {
                Icon(Icons.Filled.Close, contentDescription = "Remove ${action.title}", modifier = Modifier.size(18.dp))
            }
        }
    }
}

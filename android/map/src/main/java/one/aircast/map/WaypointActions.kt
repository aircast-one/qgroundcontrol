package one.aircast.map

import androidx.annotation.DrawableRes
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
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

data class WaypointDrop(val index: Int?, val wait: Double, val waitOffered: Boolean, val servo: Boolean, val output: Double?, val pwm: Double?, val bay: Int?)

private fun JSONObject.numberOrNull(name: String): Double? = if (isNull(name)) null else optDouble(name)

fun waypointDrop(view: JSONObject?): WaypointDrop? =
    view?.optJSONObject("drop")?.let {
        WaypointDrop(
            index = if (it.isNull("index")) null else it.optInt("index"),
            wait = it.optDouble("wait", 0.0),
            waitOffered = it.optBoolean("waitOffered"),
            servo = it.optBoolean("servo"),
            output = it.numberOrNull("output"),
            pwm = it.numberOrNull("pwm"),
            bay = it.numberOrNull("bay")?.toInt(),
        )
    }

internal const val PLACE_DROP = "core.plan.placeDrop"
internal const val REMOVE_DROP = "core.plan.removeDrop"
internal const val SET_DROP_RELEASE = "core.plan.setDropRelease"
internal const val MOVE_ACTION = "core.plan.moveAction"
internal const val SET_WAIT = "core.plan.setWait"
private const val REMOVE_ITEM = "core.plan.remove"
private const val ACTION_NOT_CHANGED = "The action was not changed."
private val WAIT_RANGE_SECONDS = 1.0..600.0
private const val DROP_NOT_CHANGED = "The drop was not changed."
private val OUTPUT_RANGE = 1.0..32.0
private val PWM_RANGE = 500.0..2500.0
private val WAIT_RANGE = 0.0..600.0
private const val PWM_STEP = 50.0

internal fun dropText(drop: WaypointDrop): String = listOfNotNull(
    drop.bay?.let { "Bay $it" },
    if (drop.output != null && drop.pwm != null) "Servo ${trimmedNumber(drop.output)} to ${trimmedNumber(drop.pwm)}" else "Gripper",
).joinToString(" \u00b7 ")

internal enum class ActionKind { Hover, Camera, Gimbal, Mode, Turn, Drop }

internal data class WaypointAction(val kind: ActionKind, val title: String, val value: String, @DrawableRes val icon: Int)

internal data class ActionOffer(val title: String, @DrawableRes val icon: Int, val kind: ActionKind, val choice: Int = -1)

private const val DEFAULT_HOVER_SECONDS = 5.0
private const val NO_CAMERA_ACTION = 0
private val HOVER_RANGE = 0.0..600.0
private val PITCH_RANGE = -90.0..30.0
private val HEADING_RANGE = -180.0..180.0
private val INTERVAL_RANGE = 1.0..3600.0
private const val ANGLE_STEP = 5.0
private const val DROP_TITLE = "Drop payload"

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

internal fun activeActions(hold: WaypointHold?, yaw: WaypointYaw?, choices: CameraChoices?, extras: CameraExtras?, drop: WaypointDrop? = null): List<WaypointAction> = listOfNotNull(
    hold?.takeIf { it.seconds > 0.0 }?.let { WaypointAction(ActionKind.Hover, "Hover", "${trimmedNumber(it.seconds)} ${it.units}", R.drawable.plan_timer) },
    choices?.takeIf { it.chosen > NO_CAMERA_ACTION }?.let { picked ->
        val label = picked.labels.getOrElse(picked.chosen) { "" }
        WaypointAction(ActionKind.Camera, sentenceCase(label), cameraInterval(extras), cameraIcon(label))
    },
    extras?.takeIf { it.commandsGimbal }?.let { WaypointAction(ActionKind.Gimbal, "Gimbal", "${degreesText(it.pitch)} / ${degreesText(it.yaw)}", R.drawable.plan_gimbal) },
    extras?.takeIf { it.modeSupported && it.commandsMode }?.let { WaypointAction(ActionKind.Mode, "Camera mode", CAMERA_MODES.getOrElse(it.mode) { "" }, R.drawable.plan_tune) },
    yaw?.degrees?.let { WaypointAction(ActionKind.Turn, "Turn aircraft", degreesText(it), R.drawable.plan_turn) },
    drop?.takeIf { it.index != null }?.let { WaypointAction(ActionKind.Drop, DROP_TITLE, dropText(it), R.drawable.plan_drop) },
)

internal fun offeredActions(hold: WaypointHold?, yaw: WaypointYaw?, choices: CameraChoices?, extras: CameraExtras?, drop: WaypointDrop? = null): List<ActionOffer> =
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
            drop?.takeIf { it.index == null }?.let { ActionOffer(DROP_TITLE, R.drawable.plan_drop, ActionKind.Drop) },
        )

private val OFFER_VERBS = listOf("Photo" to "take a photo", "Video" to "film", "Hover" to "hover", "Turn aircraft" to "turn", DROP_TITLE to "drop")

internal fun addActionsHint(offers: List<ActionOffer>): String {
    val verbs = OFFER_VERBS.filter { (title, _) -> offers.any { it.title == title } }.map { it.second }
    return when (verbs.size) {
        0 -> "Tap Add action to give the camera something to do here."
        1 -> "Tap Add action to ${verbs.single()} here."
        else -> "Tap Add action to ${verbs.dropLast(1).joinToString(", ")} or ${verbs.last()} here."
    }
}

@Composable
internal fun WaypointActions(index: Int, hold: WaypointHold?, yaw: WaypointYaw?, drop: WaypointDrop?, steps: List<ActionStep> = emptyList()) {
    var revision by remember(index) { mutableIntStateOf(0) }
    val camera by produceState<JSONObject?>(null, index, revision) {
        value = withContext(Dispatchers.Default) { ItemCameraBridge.read(index) }
    }
    val choices = cameraChoices(camera)
    val extras = cameraExtras(camera)
    val active = activeActions(hold, yaw, choices, extras, drop)
    val offers = offeredActions(hold, yaw, choices, extras, drop)
    if (active.isEmpty() && offers.isEmpty() && steps.isEmpty()) return
    val scope = rememberCoroutineScope()
    var adding by remember { mutableStateOf(false) }
    var editing by remember(index) { mutableStateOf<ActionKind?>(null) }
    var waitEditing by remember(index) { mutableStateOf<ShownAction?>(null) }
    val write: (() -> Boolean) -> Unit = { work ->
        scope.launch {
            withContext(Dispatchers.Default) { work() }
            revision += 1
        }
    }
    var refusal by remember(index) { mutableStateOf<String?>(null) }
    val refusing: (ActionKind?, () -> String?) -> Unit = { opens, work ->
        scope.launch {
            refusal = withContext(Dispatchers.Default) { work() }
            if (refusal == null && opens != null) editing = opens
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
            ActionKind.Drop -> refusing(null) { invokeRefusal(REMOVE_DROP, "[$index]", DROP_NOT_CHANGED) }
        }
    }

    Column(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("Actions", style = MaterialTheme.typography.titleSmall, modifier = Modifier.weight(1f))
            if (offers.isNotEmpty()) Box {
                FilledTonalButton(onClick = { adding = true }) {
                    Icon(painterResource(R.drawable.plan_add), contentDescription = null, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(4.dp))
                    Text("Add action")
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
                                    ActionKind.Drop -> refusing(ActionKind.Drop) { invokeRefusal(PLACE_DROP, "[$index]", DROP_NOT_CHANGED) }
                                }
                                if (offer.kind != ActionKind.Drop) editing = offer.kind
                            },
                        )
                    }
                }
            }
        }
        val ordered = orderedActions(steps, active)
        if (ordered.isEmpty()) {
            Text(addActionsHint(offers), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        if (ordered.count { it.place != null } > 1) {
            Text("Done in this order after arriving. Hold an action and drag it to change the order.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        ActionOrderList(
            waypoint = index,
            steps = ordered,
            onMove = { args -> refusing(null) { invokeRefusal(MOVE_ACTION, args, ACTION_NOT_CHANGED) } },
            onEdit = { step ->
                when {
                    step.action != null -> ({ editing = step.action.kind })
                    step.kind == WAIT && step.place != null -> ({ waitEditing = step })
                    else -> null
                }
            },
            onRemove = { step ->
                when {
                    step.action != null -> ({ remove(step.action.kind) })
                    step.item != null -> ({ refusing(null) { invokeRefusal(REMOVE_ITEM, "[${step.item}]", ACTION_NOT_CHANGED) } })
                    else -> null
                }
            },
        )
        refusal?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
    }

    waitEditing?.let { step ->
        AlertDialog(
            onDismissRequest = { waitEditing = null },
            title = { Text(WAIT) },
            text = {
                Column {
                    SettingStepper(WAIT, step.seconds ?: WAIT_RANGE_SECONDS.start, "s", 1.0, onSet = { seconds -> refusing(null) { invokeRefusal(SET_WAIT, "[$index, ${step.place}, $seconds]", ACTION_NOT_CHANGED) } }, range = WAIT_RANGE_SECONDS, slider = true)
                    Text("The aircraft holds over the waypoint this long before the next action.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            },
            confirmButton = { TextButton(onClick = { waitEditing = null }) { Text("Done") } },
        )
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
                        ActionKind.Drop -> drop?.takeIf { it.index != null }?.let { DropEditor(index, it) { work -> refusing(null, work) } }
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
private fun DropEditor(index: Int, drop: WaypointDrop, refusing: (() -> String?) -> Unit) {
    val release: (String) -> Unit = { change -> refusing { invokeRefusal(SET_DROP_RELEASE, "[$index, $change]", DROP_NOT_CHANGED) } }
    val kind: (Boolean) -> Unit = { servo -> if (servo != drop.servo) release("""{"servo":$servo}""") }
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        FilterChip(selected = !drop.servo, onClick = { kind(false) }, label = { Text("Gripper") })
        FilterChip(selected = drop.servo, onClick = { kind(true) }, label = { Text("Servo") })
    }
    if (drop.output != null && drop.pwm != null) {
        SettingStepper("Servo output", drop.output, "", 1.0, onSet = { release("""{"output":$it}""") }, range = OUTPUT_RANGE)
        SettingStepper("Release PWM", drop.pwm, "\u00b5s", PWM_STEP, onSet = { release("""{"pwm":$it}""") }, range = PWM_RANGE)
    }
    if (drop.waitOffered) SettingStepper("Wait before dropping", drop.wait, "s", 1.0, onSet = { wait -> refusing { invokeRefusal(PLACE_DROP, "[$index, $wait]", DROP_NOT_CHANGED) } }, range = WAIT_RANGE)
    Text(
        if (drop.waitOffered) "Photos and camera moves happen on arrival; the drone then waits this long over the point and drops. The servo and PWM belong to this drop, and the next drop starts from them."
        else "A plane flies through its drop without stopping. The servo and PWM belong to this drop, and the next drop starts from them.",
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
    )
}

package one.aircast.map

import androidx.annotation.DrawableRes
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInParent
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import org.json.JSONArray
import org.json.JSONObject

data class ActionStep(val kind: String, val title: String, val seconds: Double?, val item: Int?, val places: List<Int>, val refusal: String?)

private fun JSONArray?.ints(): List<Int> = this?.let { array -> (0 until array.length()).mapNotNull { at -> array.opt(at) as? Number }.map { it.toInt() } }.orEmpty()

fun actionSteps(view: JSONObject?): List<ActionStep> =
    view?.optJSONArray("actions")?.let { steps ->
        (0 until steps.length()).mapNotNull { steps.optJSONObject(it) }.map { step ->
            ActionStep(
                kind = step.optString("kind"),
                title = step.optString("title"),
                seconds = if (step.isNull("seconds")) null else step.optDouble("seconds"),
                item = if (step.isNull("item")) null else step.optInt("item"),
                places = step.optJSONArray("places").ints(),
                refusal = if (step.isNull("refusal")) null else step.optString("refusal").ifBlank { null },
            )
        }
    }.orEmpty()

internal const val HOVER = "Hover"
internal const val WAIT = "Wait"

internal data class ShownAction(
    val action: WaypointAction?,
    val kind: String,
    val title: String,
    val value: String,
    @DrawableRes val icon: Int,
    val place: Int?,
    val item: Int?,
    val seconds: Double?,
    val places: List<Int>,
    val refusal: String?,
)

private fun secondsText(seconds: Double?): String = seconds?.let { "${trimmedNumber(it)} s" }.orEmpty()

internal fun orderedActions(steps: List<ActionStep>, active: List<WaypointAction>): List<ShownAction> =
    steps.mapIndexed { place, step ->
        val action = active.firstOrNull { it.kind.name == step.kind }
        ShownAction(
            action = action,
            kind = step.kind,
            title = action?.title ?: step.title,
            value = action?.value ?: secondsText(step.seconds),
            icon = action?.icon ?: if (step.kind == WAIT) R.drawable.plan_pause else R.drawable.plan_tune,
            place = place,
            item = step.item,
            seconds = step.seconds,
            places = step.places,
            refusal = step.refusal,
        )
    } + active.filter { action -> steps.none { it.kind == action.kind.name } }.map { action ->
        ShownAction(action, action.kind.name, action.title, action.value, action.icon, null, null, null, emptyList(), null)
    }

internal fun <T> movedOrder(list: List<T>, from: Int, to: Int): List<T> =
    if (from !in list.indices || to !in list.indices) list else list.filterIndexed { at, _ -> at != from }.let { without -> without.take(to) + list[from] + without.drop(to) }

internal fun canDrop(step: ShownAction?, to: Int?): Boolean = step?.place != null && to != null && to in step.places

internal fun projectedPlaces(steps: List<ShownAction>, from: Int?, over: Int?): List<Int> {
    val identity = steps.indices.toList()
    if (from == null || over == null || !canDrop(steps.getOrNull(from), over)) return identity
    val order = movedOrder(identity, from, over)
    return identity.map { order.indexOf(it) }
}

internal fun shownAt(step: ShownAction, place: Int): ShownAction =
    if (step.kind == HOVER && place > 0) step.copy(title = WAIT, icon = R.drawable.plan_pause, value = secondsText(step.seconds)) else step

internal fun dropMove(waypoint: Int, step: ShownAction?, to: Int?): String? =
    if (to == null || step?.place == null || step.place == to || !canDrop(step, to)) null else "[$waypoint, ${step.place}, $to]"

internal fun slotUnder(centres: List<Float>, lifted: Int, dy: Float, top: Float, bottom: Float, margin: Float): Int? {
    val centre = (centres.getOrNull(lifted) ?: return null) + dy
    if (centre < top - margin || centre > bottom + margin) return null
    return centres.indices.minByOrNull { kotlin.math.abs(centres[it] - centre) }
}

internal data class ReorderDrag(val lifted: Int, val dy: Float = 0f, val over: Int? = lifted)

internal fun ReorderDrag.movedBy(dy: Float, centres: List<Float>, top: Float, bottom: Float, margin: Float): ReorderDrag =
    (this.dy + dy).let { moved -> copy(dy = moved, over = slotUnder(centres, lifted, moved, top, bottom, margin)) }

internal fun ReorderDrag.released(waypoint: Int, steps: List<ShownAction>): String? = dropMove(waypoint, steps.getOrNull(lifted), over)

private const val OUTSIDE_MARGIN_DP = 32

@Composable
internal fun ActionOrderList(
    waypoint: Int,
    steps: List<ShownAction>,
    onMove: (String) -> Unit,
    onEdit: (ShownAction) -> (() -> Unit)?,
    onRemove: (ShownAction) -> (() -> Unit)?,
) {
    val orderable = steps.filter { it.place != null }
    val fixed = steps.filter { it.place == null }
    val haptic = LocalHapticFeedback.current
    val margin = with(LocalDensity.current) { OUTSIDE_MARGIN_DP.dp.toPx() }
    val tops = remember(waypoint) { mutableStateMapOf<Int, Float>() }
    val heights = remember(waypoint) { mutableStateMapOf<Int, Float>() }
    var drag by remember(waypoint, orderable) { mutableStateOf<ReorderDrag?>(null) }
    val measured = { count: Int ->
        Triple(
            (0 until count).map { (tops[it] ?: 0f) + (heights[it] ?: 0f) / 2f },
            (0 until count).minOfOrNull { tops[it] ?: 0f } ?: 0f,
            (0 until count).maxOfOrNull { (tops[it] ?: 0f) + (heights[it] ?: 0f) } ?: 0f,
        )
    }
    val projected = projectedPlaces(orderable, drag?.lifted, drag?.over)
    val active = drag?.let { orderable.getOrNull(it.lifted) }
    val refused = drag?.let { it.over != null && it.over != it.lifted && !canDrop(active, it.over) } == true
    val outside = drag != null && drag?.over == null
    val gap = with(LocalDensity.current) { 8.dp.toPx() }
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        orderable.forEachIndexed { at, step ->
            val lifted = drag?.lifted == at
            val shift = when {
                drag == null || lifted -> 0f
                else -> ((projected.getOrNull(at) ?: at) - at) * ((heights[drag?.lifted ?: 0] ?: 0f) + gap)
            }
            val offset by animateFloatAsState(if (lifted) drag?.dy ?: 0f else shift, label = "action-slot")
            val shown = shownAt(step, projected.getOrNull(at) ?: at)
            val moveTo = { to: Int -> dropMove(waypoint, step, to)?.let { onMove(it); true } ?: false }
            Box(
                Modifier
                    .zIndex(if (lifted) 1f else 0f)
                    .graphicsLayer {
                        translationY = if (lifted) drag?.dy ?: 0f else offset
                        shadowElevation = if (lifted) 12f else 0f
                    }
                    .onGloballyPositioned { placed ->
                        if (drag == null) {
                            tops[at] = placed.positionInParent().y
                            heights[at] = placed.size.height.toFloat()
                        }
                    }
                    .holdToReorder(
                        key = listOf(waypoint, orderable),
                        onLift = {
                            haptic.performHapticFeedback(HapticFeedbackType.LongPress)
                            drag = ReorderDrag(at)
                        },
                        onDrag = { moved ->
                            val (centres, top, bottom) = measured(orderable.size)
                            val next = drag?.movedBy(moved.y, centres, top, bottom, margin)
                            if (next?.over != drag?.over) haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                            drag = next
                        },
                    ) { completed ->
                        drag?.takeIf { completed }?.released(waypoint, orderable)?.let(onMove)
                        drag = null
                    }
                    .semantics {
                        customActions = listOfNotNull(
                            (at - 1).takeIf { canDrop(step, it) }?.let { CustomAccessibilityAction("Move ${shown.title} earlier") { moveTo(it) } },
                            (at + 1).takeIf { canDrop(step, it) }?.let { CustomAccessibilityAction("Move ${shown.title} later") { moveTo(it) } },
                        )
                    },
            ) {
                ActionRow(shown, grip = true, lifted = lifted, refused = lifted && refused, onEdit = if (drag == null) onEdit(step) else null, onRemove = if (drag == null) onRemove(step) else null)
            }
        }
        if (refused) active?.refusal?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
        if (outside) Text("Let go to put it back.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        fixed.forEach { step -> ActionRow(step, grip = false, onEdit = onEdit(step), onRemove = onRemove(step)) }
    }
}

@Composable
private fun ActionRow(step: ShownAction, grip: Boolean, lifted: Boolean = false, refused: Boolean = false, onEdit: (() -> Unit)?, onRemove: (() -> Unit)?) {
    Surface(
        onClick = onEdit ?: {},
        enabled = onEdit != null,
        shape = RoundedCornerShape(14.dp),
        color = if (lifted) MaterialTheme.colorScheme.surfaceContainerHighest else MaterialTheme.colorScheme.secondaryContainer,
        contentColor = MaterialTheme.colorScheme.onSecondaryContainer,
        tonalElevation = if (lifted) 6.dp else 0.dp,
        border = if (refused) BorderStroke(2.dp, MaterialTheme.colorScheme.error) else null,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(Modifier.padding(start = if (grip) 4.dp else 12.dp), verticalAlignment = Alignment.CenterVertically) {
            if (grip) Icon(painterResource(R.drawable.plan_drag_handle), contentDescription = "Hold to reorder ${step.title}", modifier = Modifier.padding(8.dp).size(20.dp))
            Icon(painterResource(step.icon), contentDescription = null, modifier = Modifier.size(20.dp))
            Column(Modifier.weight(1f).padding(start = 8.dp, top = 6.dp, bottom = 6.dp)) {
                Text(step.title, style = MaterialTheme.typography.labelLarge)
                if (step.value.isNotBlank()) Text(step.value, style = MaterialTheme.typography.labelSmall)
            }
            onRemove?.let {
                IconButton(onClick = it, modifier = Modifier.size(40.dp)) {
                    Icon(Icons.Filled.Close, contentDescription = "Remove ${step.title}", modifier = Modifier.size(18.dp))
                }
            }
        }
    }
}

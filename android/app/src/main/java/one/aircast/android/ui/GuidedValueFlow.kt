package one.aircast.android.ui

import androidx.compose.material3.Slider
import one.aircast.android.bridge.VehicleCommands
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableDoubleStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached

internal data class GuidedReading(
    val label: String,
    val unit: String,
    val range: ClosedFloatingPointRange<Double>?,
    val initial: Double?,
    val sentence: String,
    val sendable: Boolean,
)

internal class GuidedValueKind(
    val offerId: String,
    val title: String,
    val prompt: String?,
    val commitLabel: String,
    val missingRange: String,
    val quickPicks: Boolean,
    val presets: Boolean = false,
    val explains: Boolean = true,
    val read: (Double?) -> GuidedReading?,
    val commit: (Double) -> Unit,
)

internal class OpenGuidedValue(val kind: GuidedValueKind, val reading: GuidedReading, val range: ClosedFloatingPointRange<Double>, initial: Double) {
    var target by mutableDoubleStateOf(initial)
    var settled by mutableDoubleStateOf(initial)
}

internal fun openedGuidedValue(kind: GuidedValueKind, reading: GuidedReading?): OpenGuidedValue? {
    val range = reading?.range ?: return null
    val initial = reading.initial ?: return null
    return OpenGuidedValue(kind, reading, range, initial)
}

private fun usableRange(minimum: Double?, maximum: Double?): ClosedFloatingPointRange<Double>? =
    minimum?.let { low -> maximum?.takeIf { it > low }?.let { high -> low..high } }

internal fun takeoffReading(takeoff: GuidedTakeoff?): GuidedReading? = takeoff?.let {
    GuidedReading(it.label, it.unit, usableRange(it.minimum, it.maximum), it.initial, it.sentence, sendable = true)
}

internal fun speedReading(speed: GuidedSpeed?): GuidedReading? = speed?.let {
    GuidedReading(it.label, it.unit, usableRange(it.minimum, it.maximum).takeIf { _ -> it.command != null }, it.initial, it.sentence, sendable = true)
}

internal fun altitudeReading(altitude: GuidedAltitude?): GuidedReading? = altitude?.let {
    GuidedReading(it.label, it.unit, usableRange(it.minimum, it.maximum), it.current, it.sentence, sendable = it.sends)
}

internal fun takeoffValue(offer: GuidedOffer?): GuidedValueKind = GuidedValueKind(
    offerId = "takeoff",
    title = offer?.title?.ifBlank { null } ?: "Takeoff",
    prompt = offer?.prompt,
    commitLabel = "Take off",
    missingRange = "This vehicle did not report a takeoff height range.",
    quickPicks = false,
    presets = true,
    explains = false,
    read = { target -> takeoffReading(guidedTakeoff(Qgc.get(target?.let(::guidedTakeoffPath) ?: GUIDED_TAKEOFF))) },
    commit = { target -> guidedTakeoff(Qgc.get(guidedTakeoffPath(target)))?.let { VehicleCommands.takeoff(it.targetMeters) } },
)

internal fun speedValue(offer: GuidedOffer?): GuidedValueKind = GuidedValueKind(
    offerId = "changeSpeed",
    title = offer?.title?.ifBlank { null } ?: "Change Max Ground Speed",
    prompt = offer?.prompt,
    commitLabel = "Set",
    missingRange = "This vehicle did not report a speed range.",
    quickPicks = false,
    read = { target -> speedReading(guidedSpeed(Qgc.get(target?.let(::guidedSpeedPath) ?: GUIDED_SPEED))) },
    commit = { target ->
        guidedSpeed(Qgc.get(guidedSpeedPath(target)))?.let { fresh ->
            fresh.command?.let { method -> VehicleCommands.changeSpeed(method, fresh.targetMetersSecond) }
        }
    },
)

internal fun altitudeValue(pauses: Boolean): GuidedValueKind = GuidedValueKind(
    offerId = if (pauses) PAUSE else "changeAltitude",
    title = if (pauses) "Pause" else "Change altitude",
    prompt = null,
    commitLabel = if (pauses) "Pause" else "Change",
    missingRange = "This vehicle did not report an altitude range.",
    quickPicks = true,
    read = { target -> altitudeReading(guidedAltitude(Qgc.get(target?.let { guidedAltitudePath(it, pauses) } ?: GUIDED_ALTITUDE))) },
    commit = { target ->
        guidedAltitude(Qgc.get(guidedAltitudePath(target, pauses)))?.let(::altitudeDelta)?.let { VehicleCommands.changeAltitude(it, pauses) }
    },
)

internal suspend fun openGuidedValue(kind: GuidedValueKind): OpenGuidedValue? =
    withContext(Dispatchers.Default) { openedGuidedValue(kind, kind.read(null)) }

@Composable
internal fun GuidedValueFlow(open: OpenGuidedValue, onClose: () -> Unit) {
    val kind = open.kind
    val unit = open.reading.unit
    var probe by remember(open) { mutableStateOf<GuidedReading?>(null) }
    LaunchedEffect(open, open.settled) {
        val at = open.settled
        probe = withContext(Dispatchers.Default) { kind.read(at) }
    }
    GuidedValuePanel(
        title = kind.title,
        sentence = if (kind.explains) listOfNotNull(kind.prompt, probe?.sentence).filter { it.isNotBlank() }.joinToString("\n") else "",
        commitLabel = "${kind.commitLabel} \u00b7 ${guidedValueText(open.target, unit)} $unit".trim(),
        commitEnabled = probe?.sendable == true,
        onCommit = {
            val target = open.target
            onClose()
            offMainDetached { kind.commit(target) }
        },
        onCancel = onClose,
    ) {
        val settle: (Double) -> Unit = { value ->
            open.target = value
            open.settled = value
        }
        val compact = LocalGuidedCompact.current
        val range = rangeLabel(open.range.start, open.range.endInclusive, unit)
        GuidedStepper(open.target, guidedLabel(open.reading.label, range, compact), unit, open.range.start, open.range.endInclusive, settle)
        if (kind.presets) GuidedPresets(open.target, open.range.start, open.range.endInclusive, unit, settle)
        if (!compact) Slider(
            value = open.target.toFloat(),
            onValueChange = { open.target = guidedRounded(it.toDouble(), unit) },
            onValueChangeFinished = { open.settled = open.target },
            valueRange = open.range.start.toFloat()..open.range.endInclusive.toFloat(),
        )
        if (kind.quickPicks) GuidedQuickPicks(open.target, open.range.start, open.range.endInclusive, unit, settle)
        if (!compact) range?.let { RangeHint(it) }
    }
}

internal fun guidedLabel(label: String, range: String?, compact: Boolean): String =
    if (compact && range != null) listOf(label, range).filter { it.isNotBlank() }.joinToString(" \u00b7 ") else label

package one.aircast.android.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.input.KeyboardType
import kotlin.math.abs
import kotlin.math.floor
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText

private const val SETTINGS_VIEW = "view.settings"
private const val SEARCH_SETTLE_MS = 250L

internal const val UNITS_GROUP = "unitsSettings"
internal const val VIDEO_GROUP = "videoSettings"
internal const val FLY_VIEW_GROUP = "flyViewSettings"

internal val GROUPS_WITH_A_HEAD_EDITOR = setOf(VIDEO_GROUP, FLY_VIEW_GROUP)

internal val PAGES_WITHOUT_A_SCREEN = mapOf(
    "Firmware Upgrade" to "flashing firmware needs a USB host and a bootloader dance this head does not do",
    "3D Viewer" to "there is no 3D view here to configure",
    "Flight Modes" to "twelve comma-separated lists of hidden mode names, one per airframe. The mode " +
        "picker reads what they produce; the raw lists are worse than nothing",
    "Packet Radio" to "the page needs a picker over the adapters the radio reports, and libusb " +
        "cannot enumerate on Android without a file descriptor handed in from Java",
)

internal val SECTIONS_WITHOUT_A_SCREEN = mapOf(
    "brandImageSettings" to "two paths to image files, with no way to choose a file on a phone",
    "mavlinkActionsSettings" to "two paths to JSON files that have to be on the device already",
)

internal val PAGE_NOTES = mapOf(
    "General" to "Appearance, sound, units and the defaults a new mission starts from",
    "Fly View" to "What the flight screen shows, the battery indicator and gimbal control",
    "Plan View" to "Defaults and rules for building a mission",
    "Video" to "Stream source and address, and the cameras to switch between",
    "Maps" to "Which provider draws the map, and how much imagery is kept on this device",
    "Connections" to "Serial, UDP and TCP links to the vehicle, and which kinds connect on their own",
    "MAVLink" to "Telemetry logging, how often the vehicle is asked to send each message, and forwarding",
    "ADSB Server" to "The SBS-1 receiver the traffic readout draws from",
    "Remote ID" to "Operator and aircraft identification, which some regions require in flight",
    "RTK GPS" to "Base station accuracy and position",
)

internal data class SettingsPageEntry(
    val title: String,
    val showsLinks: Boolean,
    val showsVideoSources: Boolean,
    val sectionCount: Int,
)

internal data class SettingsBlock(val title: String, val facts: List<Fact>)

internal data class SettingsSectionRows(
    val title: String,
    val group: String,
    val note: String,
    val blocks: List<SettingsBlock>,
)

internal fun settingsPagePath(title: String): String = "$SETTINGS_VIEW($title)"

internal fun inertNote(fact: Fact): String = when {
    !fact.enabled -> fact.disabledReason.ifBlank { "Has no effect yet" }
    else -> "Read-only"
}

internal fun settingsPages(view: JSONObject?): List<SettingsPageEntry> {
    val pages = view?.optJSONArray("pages") ?: return emptyList()
    return (0 until pages.length()).mapNotNull { index ->
        pages.optJSONObject(index)?.let { page ->
            SettingsPageEntry(
                title = page.optText("title"),
                showsLinks = page.optBoolean("showsLinks"),
                showsVideoSources = page.optBoolean("showsVideoSources"),
                sectionCount = page.optJSONArray("sections")?.length() ?: 0,
            )
        }
    }.filter {
        it.title.isNotBlank() && it.title !in PAGES_WITHOUT_A_SCREEN.keys &&
            (it.sectionCount > 0 || it.showsLinks)
    }
}

internal fun settingsSections(page: JSONObject?): List<SettingsSectionRows> {
    val sections = page?.optJSONArray("sections") ?: return emptyList()
    return (0 until sections.length()).mapNotNull { index ->
        sections.optJSONObject(index)?.let { section ->
            val subs = section.optJSONArray("subsections")
            SettingsSectionRows(
                title = section.optText("title"),
                group = section.optText("group"),
                note = section.optText("note"),
                blocks = (0 until (subs?.length() ?: 0)).mapNotNull { sub ->
                    subs!!.optJSONObject(sub)?.let { block ->
                        val controls = block.optJSONArray("controls")
                        SettingsBlock(
                            title = block.optText("title"),
                            facts = (0 until (controls?.length() ?: 0)).mapNotNull { control ->
                                controls!!.optJSONObject(control)?.let(::factFromControl)
                            },
                        )
                    }
                }.filter { it.facts.isNotEmpty() },
            )
        }
    }.filter { it.blocks.isNotEmpty() && it.group !in SECTIONS_WITHOUT_A_SCREEN.keys }
}

internal fun blockHeading(pageTitle: String, section: SettingsSectionRows, block: SettingsBlock): String =
    block.title.ifBlank { section.title.takeIf { it != pageTitle }.orEmpty() }

internal fun matchesIn(pageTitle: String, sections: List<SettingsSectionRows>, needle: String):
    List<SettingsSectionRows> {
    val wanted = needle.trim().lowercase()
    if (wanted.isBlank()) return emptyList()
    return sections.filterNot { it.group == UNITS_GROUP }.mapNotNull { section ->
        val hits = section.blocks.flatMap { it.facts }.filter {
            it.title.lowercase().contains(wanted) || it.name.lowercase().contains(wanted)
        }
        hits.takeIf { it.isNotEmpty() }?.let {
            section.copy(
                title = "$pageTitle \u203a ${section.title}",
                note = "",
                blocks = listOf(SettingsBlock("", it)),
            )
        }
    }
}

@Composable
fun SettingsScreen(modifier: Modifier = Modifier) {
    var pages by remember { mutableStateOf(emptyList<SettingsPageEntry>()) }
    var open by rememberSaveable { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        pages = withContext(Dispatchers.Default) { settingsPages(Qgc.get(SETTINGS_VIEW)) }
    }

    BackHandler(enabled = open != null) { open = null }

    val current = pages.firstOrNull { it.title == open }
    if (current == null) {
        SettingsList(pages, modifier) { open = it }
        return
    }

    Column(modifier.fillMaxSize()) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            IconButton(onClick = { open = null }) { Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back") }
            Text(current.title, style = MaterialTheme.typography.titleLarge)
        }
        SettingsPageBody(current, Modifier.fillMaxSize())
    }
}

@Composable
private fun SettingsList(
    pages: List<SettingsPageEntry>,
    modifier: Modifier = Modifier,
    onOpen: (String) -> Unit,
) {
    var search by rememberSaveable { mutableStateOf("") }
    var hits by remember { mutableStateOf(emptyList<SettingsSectionRows>()) }
    var searches by remember { mutableIntStateOf(0) }

    LaunchedEffect(search, pages, searches) {
        if (search.isBlank()) {
            hits = emptyList()
            return@LaunchedEffect
        }
        delay(SEARCH_SETTLE_MS)
        hits = withContext(Dispatchers.Default) {
            pages.flatMap { page ->
                matchesIn(page.title, settingsSections(Qgc.get(settingsPagePath(page.title))), search)
            }
        }
    }

    LazyColumn(modifier.fillMaxSize()) {
        item(key = "search") {
            OutlinedTextField(
                value = search,
                onValueChange = { search = it },
                label = { Text("Search settings") },
                singleLine = true,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 20.dp, vertical = 8.dp),
            )
        }

        if (search.isBlank()) {
            items(pages, key = { it.title }) { entry ->
                ListItem(
                    headlineContent = { Text(entry.title) },
                    supportingContent = { PAGE_NOTES[entry.title]?.let { Text(it) } },
                    modifier = Modifier.clickable { onOpen(entry.title) },
                )
                HorizontalDivider()
            }
            return@LazyColumn
        }

        if (hits.isEmpty()) {
            item(key = "none") { FootNote("No setting matches \"$search\".") }
            return@LazyColumn
        }

        hits.forEach { section ->
            item(key = "head${section.title}") { SectionHeader(section.title) }
            items(section.blocks.flatMap { it.facts }, key = { it.path }) { fact ->
                FactRow(fact) { searches++ }
                HorizontalDivider()
            }
        }
    }
}

@Composable
private fun SettingsPageBody(page: SettingsPageEntry, modifier: Modifier = Modifier) {
    var sections by remember(page.title) { mutableStateOf(emptyList<SettingsSectionRows>()) }
    var loaded by remember(page.title) { mutableStateOf(false) }
    var reloads by remember(page.title) { mutableIntStateOf(0) }

    LaunchedEffect(page.title, reloads) {
        sections = withContext(Dispatchers.Default) {
            settingsSections(Qgc.get(settingsPagePath(page.title)))
        }
        loaded = true
    }

    if (page.showsLinks) {
        LinksScreen(modifier) { SettingsControls(page, sections) { reloads++ } }
        return
    }

    if (!loaded) {
        Text("Reading settings.", modifier.padding(16.dp))
        return
    }

    if (sections.isEmpty()) {
        Text("No settings exposed here.", modifier.padding(16.dp))
        return
    }

    Column(modifier.verticalScroll(rememberScrollState())) {
        SettingsControls(page, sections) { reloads++ }
    }
}

@Composable
private fun SettingsControls(
    page: SettingsPageEntry,
    sections: List<SettingsSectionRows>,
    onWrite: () -> Unit,
) {
    sections.forEach { section ->
        if (section.group == UNITS_GROUP) {
            SectionHeader(section.title)
            UnitsSection()
            return@forEach
        }
        section.blocks.forEach { block ->
            blockHeading(page.title, section, block).takeIf { it.isNotBlank() }?.let { SectionHeader(it) }
            block.facts.forEach { fact ->
                FactRow(fact, onWrite = onWrite)
                HorizontalDivider()
            }
        }
        section.note
            .takeIf { it.isNotBlank() && section.group !in GROUPS_WITH_A_HEAD_EDITOR }
            ?.let { FootNote(it) }
        if (section.group == VIDEO_GROUP && page.showsVideoSources) ExtraVideoSourcesEditor()
        if (section.group == FLY_VIEW_GROUP) RcControlsEditor()
    }
}

internal fun enumLabel(fact: Fact): String =
    fact.enumStrings.getOrNull(fact.enumIndex) ?: fact.valueString

internal fun factSubtitle(fact: Fact): String = when {
    fact.enumStrings.isNotEmpty() || fact.bitmaskStrings.isNotEmpty() || fact.isBool -> ""
    else -> fact.units
}

@Composable
internal fun FactRow(
    fact: Fact,
    title: String = fact.title,
    subtitle: String = factSubtitle(fact),
    onWrite: () -> Unit = {},
) {
    val scope = rememberCoroutineScope()
    var refusal by remember(fact.path) { mutableStateOf<String?>(null) }

    fun write(block: () -> Boolean) {
        scope.launch {
            val accepted = withContext(Dispatchers.Default) { block() }
            refusal = writeRefusal(accepted)
            if (accepted) onWrite()
        }
    }

    Column {
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 64.dp)
            .padding(horizontal = 20.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Column(Modifier.weight(1f)) {
            Text(
                text = title,
                style = MaterialTheme.typography.bodyLarge,
                maxLines = 3,
                overflow = TextOverflow.Ellipsis,
            )
            if (subtitle.isNotBlank()) {
                Text(
                    text = subtitle,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }

        Box(Modifier.widthIn(max = 190.dp), contentAlignment = Alignment.CenterEnd) {
            when {
                !controlIsUnderstood(fact.controlKind) -> Column(
                    horizontalAlignment = Alignment.End,
                ) {
                    Text(
                        text = enumLabel(fact),
                        style = MaterialTheme.typography.bodyMedium,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Text(
                        text = "Edit on desktop",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                !fact.acceptsWrite -> Column(horizontalAlignment = Alignment.End) {
                    Text(
                        text = enumLabel(fact),
                        style = MaterialTheme.typography.bodyMedium,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Text(
                        text = inertNote(fact),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                fact.isBool -> Switch(
                    checked = fact.boolValue,
                    onCheckedChange = { checked -> write { Qgc.set(fact.path, checked) } },
                )
                fact.isBitmask -> BitmaskPicker(fact, ::write)
                fact.isEnum && !fact.valueIsOffTheEnumList -> EnumPicker(fact, ::write)
                else -> FactTextField(fact, onWrite)
            }
        }
    }
    refusal?.let {
        Text(
            text = it,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.error,
            modifier = Modifier.padding(start = 20.dp, bottom = 8.dp),
        )
    }
    }
}

@Composable
private fun BitmaskPicker(fact: Fact, write: (() -> Boolean) -> Unit) {
    var editing by remember(fact.path) { mutableStateOf(false) }
    val raw = bitmaskRaw(fact)

    TextButton(
        onClick = { editing = true },
        contentPadding = PaddingValues(horizontal = 4.dp, vertical = 8.dp),
    ) {
        Text(
            text = bitmaskSummary(fact),
            style = MaterialTheme.typography.bodyMedium,
            maxLines = 3,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f, fill = false),
        )
        Icon(
            imageVector = Icons.Default.KeyboardArrowDown,
            contentDescription = null,
            modifier = Modifier.padding(start = 2.dp),
        )
    }

    if (editing) {
        AlertDialog(
            onDismissRequest = { editing = false },
            title = { Text(fact.title) },
            text = {
                Column(Modifier.verticalScroll(rememberScrollState())) {
                    fact.bitmaskStrings.indices.forEach { index ->
                        val bit = fact.bitmaskValues[index]
                        Row(
                            Modifier
                                .fillMaxWidth()
                                .clickable {
                                    write { Qgc.set(fact.path, (raw xor bit).toString()) }
                                }
                                .padding(vertical = 6.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(12.dp),
                        ) {
                            Checkbox(
                                checked = raw and bit != 0L,
                                onCheckedChange = {
                                    write { Qgc.set(fact.path, (raw xor bit).toString()) }
                                },
                            )
                            Text(fact.bitmaskStrings[index], Modifier.weight(1f))
                        }
                    }
                }
            },
            confirmButton = { TextButton(onClick = { editing = false }) { Text("Done") } },
        )
    }
}

@Composable
private fun EnumPicker(fact: Fact, write: (() -> Boolean) -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    val label = enumLabel(fact)

    Column {
        TextButton(
            onClick = { expanded = true },
            contentPadding = PaddingValues(horizontal = 4.dp, vertical = 8.dp),
        ) {
            Text(
                text = label,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f, fill = false),
            )
            Icon(
                imageVector = Icons.Default.KeyboardArrowDown,
                contentDescription = null,
                modifier = Modifier.padding(start = 2.dp),
            )
        }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            fact.enumStrings.forEachIndexed { index, option ->
                DropdownMenuItem(
                    text = { Text(option) },
                    onClick = {
                        expanded = false
                        write { Qgc.set("${fact.path}.enumIndex", index) }
                    },
                )
            }
        }
    }
}

internal fun factValueLines(fact: Fact): Int = if (fact.isString) 4 else 1

internal fun truncationRefusal(fact: Fact, text: String): String? {
    if (!fact.wholeNumbersOnly) return null
    val typed = text.trim().toDoubleOrNull() ?: return null
    return if (typed == floor(typed)) null else "This setting takes whole numbers only."
}

internal fun typedValue(text: String): String = text.replace("\n", "")

internal fun factKeyboard(fact: Fact): KeyboardType = when {
    fact.isString || fact.isBool -> KeyboardType.Text
    fact.wholeNumbersOnly && fact.minString.toDoubleOrNull()?.let { it >= 0.0 } == true ->
        KeyboardType.Number
    fact.minString.toDoubleOrNull()?.let { it < 0.0 } != false -> KeyboardType.Text
    else -> KeyboardType.Decimal
}

internal fun plainNumber(text: String): String {
    val parsed = text.toDoubleOrNull() ?: return text
    if (!parsed.isFinite() || abs(parsed) >= 1e15) return text
    return when (parsed == floor(parsed)) {
        true -> parsed.toLong().toString()
        false -> parsed.toBigDecimal().stripTrailingZeros().toPlainString()
    }
}

internal fun factConstraintNote(fact: Fact): String? {
    val parts = listOfNotNull(
        fact.minString.takeIf { it.isNotBlank() && !fact.minIsDefaultForType }?.let { "Min ${plainNumber(it)}" },
        fact.maxString.takeIf { it.isNotBlank() && !fact.maxIsDefaultForType }?.let { "Max ${plainNumber(it)}" },
        fact.defaultValueString.takeIf { it.isNotBlank() }?.let { "Default ${plainNumber(it)}" },
    )
    return parts.takeIf { it.isNotEmpty() }?.joinToString(" · ")
}

internal fun factRebootNote(fact: Fact): String? = when {
    fact.vehicleRebootRequired -> "Reboot the vehicle for this to take effect."
    fact.qgcRebootRequired -> "Restart Aircast for this to take effect."
    else -> null
}

// Qgc.set returns false when the bridge refuses the write, and the reason it carries
// names a property and a class — true, and no use to a pilot. So the row says only what
// is known: the change did not take. Without this a refused switch flips back on the
// next poll and reads as the app glitching.
internal fun writeRefusal(accepted: Boolean): String? =
    if (accepted) null else "That change was not accepted."

internal fun validationMessage(result: Any?): String? =
    (result as? String)?.takeIf { it.isNotBlank() }

private suspend fun rejectionFor(fact: Fact, text: String): String? =
    truncationRefusal(fact, text) ?: withContext(Dispatchers.Default) {
        validationMessage(Qgc.invokeResult("${fact.path}.validate", text, false))
    }

@Composable
private fun FactTextField(fact: Fact, onWrite: () -> Unit) {
    var editing by remember(fact.path) { mutableStateOf<String?>(null) }
    var rejection by remember(fact.path) { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    Column {
        OutlinedTextField(
            value = editing ?: fact.valueString,
            onValueChange = {
                editing = typedValue(it)
                rejection = null
            },
            singleLine = factValueLines(fact) == 1,
            maxLines = factValueLines(fact),
            isError = rejection != null,
            keyboardOptions = KeyboardOptions(keyboardType = factKeyboard(fact)),
            modifier = Modifier.fillMaxWidth(),
            trailingIcon = {
                val committed = editing
                if (committed != null && committed != fact.valueString) {
                    TextButton(onClick = {
                        scope.launch {
                            val refused = rejectionFor(fact, committed)
                            if (refused != null) {
                                rejection = refused
                                return@launch
                            }
                            // Validation only says the value is well formed. The bridge can
                            // still refuse the write, and the fact then reads back unchanged,
                            // which is indistinguishable from a value accepted as-is.
                            val accepted = withContext(Dispatchers.Default) {
                                Qgc.set(fact.path, committed)
                            }
                            rejection = writeRefusal(accepted)
                            if (accepted) {
                                editing = null
                                onWrite()
                            }
                        }
                    }) { Text("Set") }
                }
            },
        )
        rejection?.let {
            Text(
                text = it,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
            )
        }
        if (rejection == null) {
            factConstraintNote(fact)?.let {
                Text(
                    text = it,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        factRebootNote(fact)?.let {
            Text(
                text = it,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.tertiary,
            )
        }
    }
}

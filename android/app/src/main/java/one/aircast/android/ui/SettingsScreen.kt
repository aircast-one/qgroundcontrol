package one.aircast.android.ui

import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.ui.graphics.Color
import androidx.compose.material3.FilterChip

import androidx.compose.foundation.background

import androidx.compose.ui.draw.clip

import androidx.compose.foundation.layout.aspectRatio

import androidx.annotation.DrawableRes
import one.aircast.android.R

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlin.math.abs
import kotlin.math.floor
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.aircast
import one.aircast.mapspike.optText
import org.json.JSONObject

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
)

internal val SECTIONS_WITHOUT_A_SCREEN = mapOf(
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
    "Packet Radio" to "Video and telemetry over a wfb-ng radio on a USB Wi-Fi adapter",
    "Remote ID" to "Operator and aircraft identification, which some regions require in flight",
    "RTK GPS" to "Base station accuracy and position",
)

internal enum class SettingsGroup(val title: String) { Connection("Connection"), Flying("Flying"), App("App"), More("More") }

internal data class PageLook(val group: SettingsGroup, @DrawableRes val icon: Int)

internal val PAGE_LOOKS = mapOf(
    "Connections" to PageLook(SettingsGroup.Connection, R.drawable.ic_link),
    "MAVLink" to PageLook(SettingsGroup.Connection, R.drawable.ic_swap_horiz),
    "Video" to PageLook(SettingsGroup.Connection, R.drawable.ic_videocam),
    "Packet Radio" to PageLook(SettingsGroup.Connection, R.drawable.ic_wifi),
    "ADSB Server" to PageLook(SettingsGroup.Connection, R.drawable.ic_navigation),
    "RTK GPS" to PageLook(SettingsGroup.Connection, R.drawable.ic_satellite_alt),
    "NTRIP / RTK" to PageLook(SettingsGroup.Connection, R.drawable.ic_satellite_alt),
    "Remote ID" to PageLook(SettingsGroup.Connection, R.drawable.ic_shield),
    "Fly View" to PageLook(SettingsGroup.Flying, R.drawable.ic_flight),
    "Plan View" to PageLook(SettingsGroup.Flying, R.drawable.ic_route),
    "Maps" to PageLook(SettingsGroup.Flying, R.drawable.ic_map),
    "Flight Modes" to PageLook(SettingsGroup.Flying, R.drawable.ic_toggle_on),
    "3D Viewer" to PageLook(SettingsGroup.Flying, R.drawable.ic_explore),
    "General" to PageLook(SettingsGroup.App, R.drawable.ic_tune),
    "PX4 Log Transfer" to PageLook(SettingsGroup.App, R.drawable.ic_download),
    "Firmware Upgrade" to PageLook(SettingsGroup.App, R.drawable.ic_developer_board),
    "Console" to PageLook(SettingsGroup.App, R.drawable.ic_terminal),
    "About" to PageLook(SettingsGroup.App, R.drawable.ic_description),
)

internal fun pageLook(title: String): PageLook = PAGE_LOOKS[title] ?: PageLook(SettingsGroup.More, R.drawable.ic_settings)

internal fun groupedPages(pages: List<SettingsPageEntry>): List<Pair<SettingsGroup, List<SettingsPageEntry>>> =
    pages.groupBy { pageLook(it.title).group }.toList().sortedBy { it.first.ordinal }

internal data class HelpLink(val name: String, val url: String, val host: String)

internal data class SettingsPageEntry(
    val title: String,
    val showsLinks: Boolean,
    val showsVideoSources: Boolean,
    val sectionCount: Int,
    val showsAbout: Boolean = false,
    val showsConsole: Boolean = false,
    val showsNtrip: Boolean = false,
    val showsPx4Logs: Boolean = false,
    val showsPacketRadio: Boolean = false,
    val helpLinks: List<HelpLink> = emptyList(),
)

internal data class SettingsBlock(val title: String, val facts: List<Fact>)

internal data class SettingsSectionRows(
    val title: String,
    val group: String,
    val note: String,
    val blocks: List<SettingsBlock>,
)

internal fun settingsPagePath(title: String): String = "$SETTINGS_VIEW($title)"

internal val NOT_BUILT_HERE = mapOf(
    "displayPresetsTabFirst" to "this head has no presets tab",
    "maxCacheMemorySize" to "the map keeps its own memory cache",
)

internal fun notBuiltHere(fact: Fact): String? = NOT_BUILT_HERE[fact.name]

internal fun editOnDesktop(fact: Fact): Boolean =
    !controlIsUnderstood(fact.controlKind) && notBuiltHere(fact) == null

internal fun inertNote(fact: Fact): String = when {
    !fact.enabled -> fact.disabledReason.ifBlank { "Has no effect yet" }
    notBuiltHere(fact) != null -> "No effect here - ${notBuiltHere(fact)}"
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
                showsAbout = page.optBoolean("showsAbout"),
                showsConsole = page.optBoolean("showsConsole"),
                showsNtrip = page.optBoolean("showsNtrip"),
                showsPx4Logs = page.optBoolean("showsPx4Logs"),
                showsPacketRadio = page.optBoolean("showsPacketRadio"),
                helpLinks = page.optJSONArray("helpLinks")?.let { links ->
                    (0 until links.length()).mapNotNull { i ->
                        links.optJSONObject(i)?.let { HelpLink(it.optText("name"), it.optText("url"), it.optText("host")) }
                    }
                } ?: emptyList(),
            )
        }
    }.filter {
        it.title.isNotBlank() && it.title !in PAGES_WITHOUT_A_SCREEN.keys &&
            (it.sectionCount > 0 || it.showsLinks || it.showsAbout || it.showsConsole || it.showsPx4Logs)
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
    LaunchedEffect(AppNavigation.settingsPage) {
        AppNavigation.settingsPage?.let { requested ->
            open = requested
            AppNavigation.settingsPage = null
        }
    }

    LaunchedEffect(Unit) {
        pages = withContext(Dispatchers.Default) { settingsPages(Qgc.get(SETTINGS_VIEW)) }
    }

    BackHandler(enabled = open != null) { open = null }

    val current = pages.firstOrNull { it.title == open }
    BoxWithConstraints(modifier.fillMaxSize()) {
        if (maxWidth >= LIST_DETAIL_MIN_WIDTH) {
            Row(Modifier.fillMaxSize()) {
                SettingsList(pages, Modifier.width(LIST_PANE_WIDTH).background(MaterialTheme.colorScheme.surfaceContainerLow), selected = open) { open = it }
                Box(Modifier.weight(1f).fillMaxHeight()) {
                    if (current == null) {
                        EmptyState(R.drawable.ic_settings, "Settings", "Choose a group on the left.")
                    } else {
                        Column(Modifier.fillMaxSize()) {
                            PageTopBar(current.title, "Back") { open = null }
                            SettingsPageBody(current, Modifier.fillMaxSize())
                        }
                    }
                }
            }
        } else if (current == null) {
            SettingsList(pages) { open = it }
        } else {
            Column(Modifier.fillMaxSize()) {
                PageTopBar(current.title, "Back") { open = null }
                SettingsPageBody(current, Modifier.fillMaxSize())
            }
        }
    }
}

internal val LIST_DETAIL_MIN_WIDTH = 840.dp
internal val LIST_PANE_WIDTH = 380.dp

@Composable
private fun SettingsList(
    pages: List<SettingsPageEntry>,
    modifier: Modifier = Modifier,
    selected: String? = null,
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
        item(key = "title") {
            Text(
                "Settings",
                style = MaterialTheme.typography.headlineMedium,
                modifier = Modifier.padding(start = 16.dp, end = 16.dp, top = 20.dp, bottom = 4.dp),
            )
        }
        item(key = "search") {
            SearchPill(search, { search = it }, "Search settings")
        }

        if (search.isBlank()) {
            groupedPages(pages).forEach { (group, entries) ->
                item(key = "group${group.name}") { SectionHeader(group.title) }
                items(entries, key = { it.title }) { entry ->
                    SetupRow(
                        title = entry.title,
                        subtitle = PAGE_NOTES[entry.title].orEmpty(),
                        onClick = { onOpen(entry.title) },
                        icon = pageLook(entry.title).icon,
                        selected = entry.title == selected,
                    )
                }
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

    if (page.showsAbout) {
        AboutPage(page.helpLinks, modifier)
        return
    }

    if (page.showsConsole) {
        AppLogPage(modifier)
        return
    }

    if (page.showsPx4Logs) {
        Px4LogTransferPage(modifier)
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
        if (page.showsVideoSources) {
            VideoSurface(
                Modifier
                    .padding(horizontal = 16.dp, vertical = 8.dp)
                    .fillMaxWidth()
                    .aspectRatio(16f / 9f)
                    .clip(MaterialTheme.shapes.large)
                    .background(MaterialTheme.colorScheme.surfaceContainerHighest),
                expanded = true,
            )
        }
        SettingsControls(page, sections) { reloads++ }
        if (page.title == GENERAL_PAGE) ResetAllSettingsRow()
    }
}

@Composable
private fun SettingsControls(
    page: SettingsPageEntry,
    sections: List<SettingsSectionRows>,
    onWrite: () -> Unit,
) {
    if (page.showsNtrip) NtripStatusSection(onWrite)
    if (page.showsPacketRadio) PacketRadioSection(onWrite)
    sections.forEach { section ->
        if (section.group == UNITS_GROUP) {
            SectionHeader(section.title)
            UnitsSection()
            return@forEach
        }
        section.blocks.forEach { block ->
            blockHeading(page.title, section, block).takeIf { it.isNotBlank() }?.let { SectionHeader(it) }
            FactRuns(block.facts, onWrite)
            if (page.showsNtrip && block.title == NTRIP_MOUNTPOINT_BLOCK) NtripMountpointBrowser(onWrite)
        }
        section.note
            .takeIf { it.isNotBlank() && section.group !in GROUPS_WITH_A_HEAD_EDITOR }
            ?.let { FootNote(it) }
        if (section.group == VIDEO_GROUP && page.showsVideoSources) ExtraVideoSourcesEditor()
        if (section.group == FLY_VIEW_GROUP) RcControlsEditor()
        if (section.group == OFFLINE_MAPS_GROUP) OfflineMapsSection()
        if (section.group == MAVLINK_GROUP) SigningKeysSection()
        if (section.group == MAVLINK_GROUP) LinkStatusSection()
        if (section.group == MAVLINK_ACTIONS_GROUP) MavlinkActionsSection(onWrite)
    }
}

@Composable
internal fun FactRuns(facts: List<Fact>, onWrite: () -> Unit = {}) {
    fieldRuns(facts).forEach { run ->
        if (run.size == 1) {
            FactRow(run.first(), onWrite = onWrite)
        } else {
            Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                run.forEach { fact ->
                    FactRow(fact, subtitle = factSubtitle(fact), fieldModifier = Modifier.weight(1f).padding(vertical = 8.dp), onWrite = onWrite)
                }
            }
        }
    }
}

internal const val NTRIP_MOUNTPOINT_BLOCK = "Mountpoint"

internal fun isSecret(fact: Fact): Boolean = fact.name.endsWith("Password", ignoreCase = true)

internal fun enumLabel(fact: Fact): String =
    fact.enumStrings.getOrNull(fact.enumIndex) ?: fact.valueString

internal fun factSubtitle(fact: Fact): String = listOfNotNull(
    when {
        fact.enumStrings.isNotEmpty() || fact.bitmaskStrings.isNotEmpty() || fact.isBool -> ""
        else -> fact.units
    }.ifBlank { null },
    notBuiltHere(fact)?.let { "No effect here - $it" },
).joinToString(" · ")

@Composable
internal fun FactRow(
    fact: Fact,
    title: String = fact.heading,
    subtitle: String = listOf(fact.detail, factSubtitle(fact)).filter { it.isNotBlank() }.joinToString(" · "),
    titleColor: Color = Color.Unspecified,
    fieldModifier: Modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
    onWrite: () -> Unit = {},
) {
    val scope = rememberCoroutineScope()
    var refusal by remember(fact.path) { mutableStateOf<String?>(null) }

    val segmented = !editOnDesktop(fact) && notBuiltHere(fact) == null && !fact.isBitmask &&
        showsAsSegments(fact.isEnum, fact.valueIsOffTheEnumList, fact.acceptsWrite, fact.enumStrings)
    val asField = !segmented && showsAsField(fact)

    fun write(block: () -> Boolean) {
        scope.launch {
            val accepted = withContext(Dispatchers.Default) { block() }
            refusal = writeRefusal(accepted)
            if (accepted) onWrite()
        }
    }

    if (asField) {
        Column(fieldModifier) {
            val inside = title.takeIf { it.length <= FIELD_LABEL_BUDGET }
            if (inside == null) Text(title, style = MaterialTheme.typography.bodyLarge, color = titleColor, modifier = Modifier.padding(bottom = 8.dp))
            when {
                fact.isBitmask -> BitmaskPicker(fact, ::write, inside)
                fact.isEnum && !fact.valueIsOffTheEnumList -> EnumField(fact, inside, ::write)
                else -> FactTextField(fact, onWrite, inside)
            }
            val note = subtitle.split(" · ").filter { it.isNotBlank() && it != fact.units }.joinToString(" · ")
            if (note.isNotBlank()) {
                Text(
                    text = note,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(start = 16.dp, top = 4.dp),
                )
            }
            refusal?.let {
                Text(text = it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(start = 16.dp, top = 4.dp))
            }
        }
        return
    }

    Column {
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 64.dp)
            .padding(horizontal = 16.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Column(Modifier.weight(1f)) {
            Text(
                text = title,
                style = MaterialTheme.typography.bodyLarge,
                color = titleColor,
                maxLines = 3,
                overflow = TextOverflow.Ellipsis,
            )
            val rowNote = subtitle.split(" · ").filter { it.isNotBlank() && it != fact.units }.joinToString(" · ")
            if (rowNote.isNotBlank() && !rowNote.equals(title, ignoreCase = true)) {
                Text(
                    text = rowNote,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            if (!fact.acceptsWrite && notBuiltHere(fact) == null && !editOnDesktop(fact)) {
                Text(
                    text = inertNote(fact),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }

        if (!segmented) Box(Modifier.widthIn(max = 190.dp), contentAlignment = Alignment.CenterEnd) {
            when {
                editOnDesktop(fact) -> Column(
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
                !fact.acceptsWrite || notBuiltHere(fact) != null -> Column(horizontalAlignment = Alignment.End) {
                    if (fact.isBool) {
                        Switch(checked = fact.boolValue, onCheckedChange = null, enabled = false)
                    } else {
                        Text(
                            text = listOf(enumLabel(fact), fact.units.takeIf { !fact.isEnum }.orEmpty()).filter { it.isNotBlank() }.joinToString(" "),
                            style = MaterialTheme.typography.bodyMedium,
                            maxLines = 2,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
                fact.isBool -> Switch(
                    checked = fact.boolValue,
                    onCheckedChange = { checked -> write { Qgc.set(fact.path, checked) } },
                )
                else -> BitmaskPicker(fact, ::write)
            }
        }
    }
    if (segmented) {
        Row(
            Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, bottom = 10.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            fact.enumStrings.forEachIndexed { index, option ->
                FilterChip(
                    selected = index == fact.enumIndex,
                    onClick = { write { Qgc.set("${fact.path}.enumIndex", index) } },
                    label = { Text(option) },
                )
            }
        }
    }
    refusal?.let {
        Text(
            text = it,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.error,
            modifier = Modifier.padding(start = 16.dp, bottom = 8.dp),
        )
    }
    }
}

internal const val SEGMENT_LABEL_BUDGET = 28
internal const val PAIRED_LABEL_BUDGET = 20
internal const val FIELD_LABEL_BUDGET = 40
internal const val PAIRED_OPTION_BUDGET = 16
internal const val PAIRED_UNITS_BUDGET = 6

internal fun showsAsField(fact: Fact): Boolean =
    fact.acceptsWrite && notBuiltHere(fact) == null && !editOnDesktop(fact) && !fact.isBool

internal fun pairsAsField(fact: Fact): Boolean =
    fact.shortLabel.length in 1..PAIRED_LABEL_BUDGET && showsAsField(fact) && when {
        fact.isEnum -> !fact.valueIsOffTheEnumList &&
            !showsAsSegments(fact.isEnum, fact.valueIsOffTheEnumList, fact.acceptsWrite, fact.enumStrings) &&
            fact.enumStrings.all { it.length <= PAIRED_OPTION_BUDGET }
        else -> !fact.isString && !fact.isBitmask && fact.enumStrings.isEmpty() && fact.units.length <= PAIRED_UNITS_BUDGET
    }

internal fun fieldRuns(facts: List<Fact>, pairable: (Fact) -> Boolean = { true }): List<List<Fact>> =
    facts.fold(emptyList()) { runs, fact ->
        val last = runs.lastOrNull()
        val pairs = { candidate: Fact -> pairsAsField(candidate) && pairable(candidate) }
        if (last != null && last.size == 1 && pairs(last.first()) && pairs(fact)) runs.dropLast(1) + listOf(last + fact) else runs + listOf(listOf(fact))
    }

internal fun showsAsSegments(isEnum: Boolean, offList: Boolean, writable: Boolean, options: List<String>): Boolean =
    isEnum && !offList && writable && options.size in 2..4 && options.sumOf { it.length } <= SEGMENT_LABEL_BUDGET

internal fun bitmaskToggled(fact: Fact, raw: Long, index: Int): Long {
    val bit = fact.bitmaskValues[index]
    val all = fact.firstEntryIsAll && index == 0
    return when {
        raw and bit != 0L -> raw and bit.inv()
        all -> fact.bitmaskValues.drop(1).fold(raw) { value, other -> value and other.inv() } or bit
        else -> raw or bit
    }
}

internal fun bitmaskEntryEnabled(fact: Fact, raw: Long, index: Int): Boolean =
    !(fact.firstEntryIsAll && index > 0 && fact.bitmaskValues.isNotEmpty() && raw and fact.bitmaskValues[0] != 0L)

@Composable
private fun BitmaskPicker(fact: Fact, write: (() -> Boolean) -> Unit, label: String? = null) {
    var editing by remember(fact.path) { mutableStateOf(false) }
    val raw = bitmaskRaw(fact)

    Box {
        OutlinedTextField(
            value = bitmaskSummary(fact),
            onValueChange = {},
            readOnly = true,
            maxLines = 3,
            label = label?.let { { Text(it, maxLines = 1, overflow = TextOverflow.Ellipsis) } },
            trailingIcon = { Icon(Icons.Default.KeyboardArrowDown, contentDescription = null) },
            modifier = Modifier.fillMaxWidth(),
        )
        Box(Modifier.matchParentSize().clickable { editing = true })
    }

    if (editing) {
        AlertDialog(
            onDismissRequest = { editing = false },
            title = { Text(fact.title) },
            text = {
                Column(Modifier.verticalScroll(rememberScrollState())) {
                    fact.bitmaskStrings.indices.forEach { index ->
                        val bit = fact.bitmaskValues[index]
                        val live = bitmaskEntryEnabled(fact, raw, index)
                        Row(
                            Modifier
                                .fillMaxWidth()
                                .clickable(enabled = live) {
                                    write { Qgc.set(fact.path, bitmaskToggled(fact, raw, index).toString()) }
                                }
                                .padding(vertical = 6.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(12.dp),
                        ) {
                            Checkbox(
                                checked = raw and bit != 0L,
                                enabled = live,
                                onCheckedChange = {
                                    write { Qgc.set(fact.path, bitmaskToggled(fact, raw, index).toString()) }
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
private fun EnumField(fact: Fact, label: String?, write: (() -> Boolean) -> Unit) {
    ChoiceField(label, enumLabel(fact), fact.enumStrings) { index ->
        // qtpaths: settings.appSettings.indoorPalette.enumIndex, vehicle.parameterManager.getParameter(-1,RTL_TYPE).enumIndex
        write { Qgc.set("${fact.path}.enumIndex", index) }
    }
}

@Composable
internal fun ChoiceField(label: String?, value: String, options: List<String>, modifier: Modifier = Modifier, onPick: (Int) -> Unit) {
    var expanded by remember { mutableStateOf(false) }

    Box(modifier) {
        OutlinedTextField(
            value = value,
            onValueChange = {},
            readOnly = true,
            singleLine = true,
            label = label?.let { { Text(it, maxLines = 1, overflow = TextOverflow.Ellipsis) } },
            trailingIcon = { Icon(Icons.Default.KeyboardArrowDown, contentDescription = null) },
            modifier = Modifier.fillMaxWidth(),
        )
        Box(Modifier.matchParentSize().clickable { expanded = true })
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            options.forEachIndexed { index, option ->
                DropdownMenuItem(
                    text = { Text(option) },
                    onClick = {
                        expanded = false
                        onPick(index)
                    },
                )
            }
        }
    }
}

internal fun factValueLines(fact: Fact): Int = if (fact.isString) 4 else 1

internal fun truncationRefusal(fact: Fact, text: String): String? {
    if (!fact.wholeNumbersOnly) return null
    val typed = text.trim().toDoubleOrNull()?.takeIf { it.isFinite() } ?: return null
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
    fact.vehicleRebootRequired -> "Reboot vehicle for changes to take effect."
    fact.qgcRebootRequired -> "Restart Aircast for this to take effect."
    else -> null
}

internal fun writeRefusal(accepted: Boolean): String? =
    if (accepted) null else "That change was not accepted."

internal fun validationMessage(result: Any?): String? =
    (result as? String)?.takeIf { it.isNotBlank() }

private suspend fun rejectionFor(fact: Fact, text: String): String? =
    truncationRefusal(fact, text) ?: withContext(Dispatchers.Default) {
        // qtpaths: settings.appSettings.indoorPalette.validate, vehicle.parameterManager.getParameter(-1,RTL_ALT).validate
        validationMessage(Qgc.invokeResult("${fact.path}.validate", text, false))
    }

@Composable
private fun FactTextField(fact: Fact, onWrite: () -> Unit, label: String?) {
    var editing by remember(fact.path) { mutableStateOf<String?>(null) }
    var rejection by remember(fact.path) { mutableStateOf<String?>(null) }
    var revealed by remember(fact.path) { mutableStateOf(false) }
    val secret = isSecret(fact)
    val scope = rememberCoroutineScope()

    Column {
        OutlinedTextField(
            value = editing ?: fact.valueString,
            label = label?.let { { Text(it, maxLines = 1, overflow = TextOverflow.Ellipsis) } },
            suffix = fact.units.takeIf { it.isNotBlank() }?.let { { Text(it) } },
            visualTransformation = if (secret && !revealed) androidx.compose.ui.text.input.PasswordVisualTransformation() else androidx.compose.ui.text.input.VisualTransformation.None,
            leadingIcon = if (secret) {
                { TextButton(onClick = { revealed = !revealed }) { Text(if (revealed) "Hide" else "Show") } }
            } else {
                null
            },
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
                modifier = Modifier.padding(start = 16.dp, top = 4.dp),
            )
        }
        if (rejection == null) {
            factConstraintNote(fact)?.let {
                Text(
                    text = it,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(start = 16.dp, top = 4.dp),
                )
            }
        }
        factRebootNote(fact)?.let {
            Text(
                text = it,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.aircast.warning,
                modifier = Modifier.padding(start = 16.dp, top = 4.dp),
            )
        }
    }
}

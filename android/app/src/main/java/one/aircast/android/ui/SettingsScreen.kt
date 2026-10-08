package one.aircast.android.ui

import one.aircast.android.bridge.qgcDouble
import one.aircast.android.bridge.settingControl
import one.aircast.android.bridge.qgcPath
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.size
import androidx.compose.ui.res.painterResource
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.ui.graphics.Color
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow

import androidx.compose.foundation.background

import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate

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
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.snapshotFlow
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.filter
import one.aircast.android.bridge.offMainDetached
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.DisposableEffect
import androidx.compose.ui.semantics.Role
import androidx.compose.foundation.selection.toggleable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.foundation.interaction.collectIsFocusedAsState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.material3.LocalTextStyle
import androidx.compose.ui.unit.dp
import kotlin.math.abs
import kotlin.math.floor
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.FactSlider
import one.aircast.android.bridge.Qgc
import one.aircast.map.aircast
import one.aircast.map.optText
import org.json.JSONObject

private const val SETTINGS_VIEW = "view.settings"
private const val SEARCH_SETTLE_MS = 250L
private const val PREVIEW_HEIGHT_SHARE = 0.4f

internal const val UNITS_GROUP = "unitsSettings"
internal const val VIDEO_GROUP = "videoSettings"
internal const val FLY_VIEW_GROUP = "flyViewSettings"
internal const val VIEWER_3D_GROUP = "viewer3DSettings"

internal val GROUPS_WITH_A_HEAD_EDITOR = setOf(VIDEO_GROUP, FLY_VIEW_GROUP)

internal val PAGES_WITHOUT_A_SCREEN = mapOf(
    "Firmware Upgrade" to "QGC has no such settings page; the firmware screen reads these settings itself",
    "Flight Modes" to "twelve comma-separated lists of hidden mode names, one per airframe. The mode " +
        "picker reads what they produce; the raw lists are worse than nothing",
)

internal val SECTIONS_DRAWN_BY_THE_HEAD = setOf(MAVLINK_ACTIONS_GROUP)

internal const val CONNECTIONS_PAGE = "Connections"

private val PAGE_GLANCES = mapOf(
    "Maps" to listOf("settings.flightMapSettings.mapProvider", "settings.flightMapSettings.mapType"),
)

internal fun glanceText(displays: List<String>): String = displays.filter { it.isNotBlank() }.map(::sentenceCase).joinToString(" · ")

@Composable
private fun pageGlance(title: String): String {
    if (title == "About") return "Aircast ${qgcVersion()}"
    if (title == GENERAL_PAGE) {
        val system by qgcDouble("settings.unitsSettings.unitSystem")
        return if (system.isNaN()) "" else unitSystemLabel(system.toInt())
    }
    if (title == VIDEO_PAGE) {
        val cameras by qgcPath(CAMERAS_VIEW)
        return camerasGlance(camerasReading(cameras))
    }
    val paths = PAGE_GLANCES[title] ?: return ""
    val displays = paths.map { path ->
        val json by qgcPath(settingControl(path))
        json?.optText("display").orEmpty()
    }
    return glanceText(displays)
}

private val PAGE_TITLES = mapOf(CONNECTIONS_PAGE to "Links", "ADSB Server" to "ADS-B server")

internal fun pageTitle(title: String): String = PAGE_TITLES[title] ?: sentenceCase(title)

internal fun activeLinkCount(view: JSONObject?): Int =
    view?.optJSONArray("links")?.let { links -> (0 until links.length()).count { links.optJSONObject(it)?.optBoolean("connected") == true } } ?: 0

internal fun activeLinksGlance(view: JSONObject?): String =
    view?.optJSONArray("links")?.let { links ->
        (0 until links.length()).mapNotNull { links.optJSONObject(it) }.filter { it.optBoolean("connected") }
            .map { it.optText("summary").ifBlank { it.optText("name") } }.filter { it.isNotBlank() }.distinct().joinToString(" \u00b7 ")
    }.orEmpty()

internal fun activeLinksText(count: Int): String = if (count > 0) "$count active" else ""

internal enum class SettingsGroup(val title: String) { Safety("Safety"), Control("Control"), Camera("Camera"), Transmission("Transmission"), General("General") }

internal data class PageLook(val group: SettingsGroup, @DrawableRes val icon: Int, val inline: Boolean = false, val tabHome: Boolean = false)

internal val PAGE_LOOKS = mapOf(
    "ADSB Server" to PageLook(SettingsGroup.Safety, R.drawable.ic_navigation, inline = true),
    "Remote ID" to PageLook(SettingsGroup.Safety, R.drawable.ic_shield),
    "Fly View" to PageLook(SettingsGroup.Control, R.drawable.ic_flight, inline = true),
    "Flight Modes" to PageLook(SettingsGroup.Control, R.drawable.ic_toggle_on),
    "Plan View" to PageLook(SettingsGroup.Control, R.drawable.ic_route, inline = true),
    "Maps" to PageLook(SettingsGroup.Control, R.drawable.ic_map),
    "3D Viewer" to PageLook(SettingsGroup.Control, R.drawable.ic_explore),
    "Video" to PageLook(SettingsGroup.Camera, R.drawable.ic_videocam, inline = true),
    "Connections" to PageLook(SettingsGroup.Transmission, R.drawable.ic_link),
    "MAVLink" to PageLook(SettingsGroup.Transmission, R.drawable.ic_swap_horiz),
    "Packet Radio" to PageLook(SettingsGroup.Transmission, R.drawable.ic_wifi),
    "RTK GPS" to PageLook(SettingsGroup.Transmission, R.drawable.ic_satellite_alt),
    "NTRIP / RTK" to PageLook(SettingsGroup.Transmission, R.drawable.ic_satellite_alt),
    "General" to PageLook(SettingsGroup.General, R.drawable.ic_tune, inline = true, tabHome = true),
    "PX4 Log Transfer" to PageLook(SettingsGroup.General, R.drawable.ic_download),
    "Firmware Upgrade" to PageLook(SettingsGroup.General, R.drawable.ic_developer_board),
    "App Logging" to PageLook(SettingsGroup.General, R.drawable.ic_description),
    "Console" to PageLook(SettingsGroup.General, R.drawable.ic_terminal),
    "About" to PageLook(SettingsGroup.General, R.drawable.ic_help),
)

internal val BLOCK_HOMES = mapOf(("Fly View" to "Guided Commands") to SettingsGroup.Safety)

internal fun pageLook(title: String): PageLook = PAGE_LOOKS[title] ?: PageLook(SettingsGroup.General, R.drawable.ic_settings)

internal fun blockGroup(page: String, block: SettingsBlock): SettingsGroup = BLOCK_HOMES[page to block.title] ?: pageLook(page).group

internal fun tabPages(group: SettingsGroup, pages: List<SettingsPageEntry>): List<SettingsPageEntry> {
    val lends = BLOCK_HOMES.filterValues { it == group }.keys.map { it.first }
    val own = pages.filter { pageLook(it.title).group == group }
        .sortedBy { PAGE_LOOKS.keys.indexOf(it.title).takeIf { at -> at >= 0 } ?: Int.MAX_VALUE }
    return pages.filter { it.title in lends && pageLook(it.title).group != group } + own
}

internal fun sectionsIn(group: SettingsGroup, page: String, sections: List<SettingsSectionRows>): List<SettingsSectionRows> =
    sections.map { section -> section.copy(blocks = section.blocks.filter { blockGroup(page, it) == group }) }.filter { it.blocks.isNotEmpty() }

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
    val keywords: String = "",
)

internal data class SettingsBlock(val title: String, val facts: List<Fact>)

internal data class SettingsSectionRows(
    val title: String,
    val group: String,
    val note: String,
    val blocks: List<SettingsBlock>,
)

internal fun settingsPagePath(title: String): String = "$SETTINGS_VIEW($title)"

internal fun editOnDesktop(fact: Fact): Boolean = !controlIsUnderstood(fact.controlKind)

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
                showsAbout = page.optBoolean("showsAbout"),
                showsConsole = page.optBoolean("showsConsole"),
                showsNtrip = page.optBoolean("showsNtrip"),
                showsPx4Logs = page.optBoolean("showsPx4Logs"),
                showsPacketRadio = page.optBoolean("showsPacketRadio"),
                keywords = page.optText("keywords"),
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
                                controls!!.optJSONObject(control)?.let(::factFromControl)?.let(::paletteNamed)
                            },
                        )
                    }
                }.filter { it.facts.isNotEmpty() },
            )
        }
    }.map { if (it.group in SECTIONS_DRAWN_BY_THE_HEAD) it.copy(blocks = listOf(SettingsBlock("", emptyList()))) else it }
        .filter { it.blocks.isNotEmpty() }
}

internal fun blockHeading(pageTitle: String, section: SettingsSectionRows, block: SettingsBlock): String =
    block.title.ifBlank { section.title.takeIf { it != pageTitle }.orEmpty() }

internal fun shownBreadcrumb(title: String): String =
    title.split(" \u203a ").joinToString(" \u203a ", transform = ::sentenceCase)

internal fun pageMatches(page: SettingsPageEntry, needle: String): Boolean =
    needle.trim().lowercase().let { wanted -> wanted.isNotEmpty() && (page.title.lowercase().contains(wanted) || page.keywords.contains(wanted)) }

internal fun matchesIn(pageTitle: String, sections: List<SettingsSectionRows>, needle: String):
    List<SettingsSectionRows> {
    val wanted = needle.trim().lowercase()
    if (wanted.isBlank()) return emptyList()
    return sections.filterNot { it.group == UNITS_GROUP }.mapNotNull { section ->
        val hits = section.blocks.flatMap { it.facts }.filter {
            it.title.lowercase().contains(wanted) || it.name.lowercase().contains(wanted) || it.keywords.contains(wanted)
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

internal const val AIRCRAFT_SETUP = "Aircraft setup"

internal fun tabSetupPages(group: SettingsGroup): List<String> = when (group) {
    SettingsGroup.Safety -> listOf(SAFETY_SETUP_PAGE, SENSORS)
    SettingsGroup.Control -> listOf(FLIGHT_MODES_PAGE)
    else -> emptyList()
}

internal fun tabSetupComponents(group: SettingsGroup, components: List<SetupComponent>): List<SetupComponent> =
    tabSetupPages(group).mapNotNull { name -> components.firstOrNull { it.name == name } }

internal fun setupSearchHits(components: List<SetupComponent>, query: String): List<SetupComponent> =
    if (query.isBlank()) emptyList() else components.filter { setupMatches(it.name, query) }

@Composable
private fun TabSetupRows(group: SettingsGroup, onOpenSetup: (String?) -> Unit) {
    val setupJson by qgcPath(SETUP)
    val components = remember(setupJson) { setupComponents(setupJson) }
    tabSetupComponents(group, components).map { component ->
        SetupRow(title = sentenceCase(component.name), status = "", icon = setupIcon(component.known, component.className), onClick = { onOpenSetup(component.name) })
    }
}

@Composable
private fun AircraftSetupRow(onOpenSetup: (String?) -> Unit) {
    SetupRow(title = AIRCRAFT_SETUP, status = "", icon = R.drawable.ic_build, onClick = { onOpenSetup(null) })
}

@Composable
internal fun SettingsScreen(group: SettingsGroup, initialPage: String?, modifier: Modifier = Modifier, onOpenSetup: (String?) -> Unit = {}) {
    var everyPage by remember { mutableStateOf(emptyList<SettingsPageEntry>()) }
    var open by rememberSaveable(group) { mutableStateOf(initialPage?.takeUnless { pageLook(it).inline }) }

    LaunchedEffect(Unit) {
        everyPage = withContext(Dispatchers.Default) { settingsPages(Qgc.get(SETTINGS_VIEW)) }
    }

    BackHandler(enabled = open != null) { open = null }

    val current = everyPage.firstOrNull { it.title == open }
    val heading = remember { mutableStateOf<PageHeading?>(null) }
    val headingBack: () -> Unit = { heading.value?.back?.invoke() ?: run { open = null } }
    CompositionLocalProvider(LocalPageHeading provides heading, LocalDetailBehindHelp provides true) {
        Box(modifier.fillMaxSize(), contentAlignment = Alignment.TopCenter) {
            val column = Modifier.fillMaxHeight().widthIn(max = DETAIL_PANE_MAX_WIDTH)
            if (current == null) {
                SettingsTab(group, everyPage, column, onOpenSetup) { open = it }
            } else {
                Column(column) {
                    PageTopBar(heading.value?.title ?: pageTitle(current.title), "Back", headingBack)
                    SettingsPageBody(current, Modifier.fillMaxSize())
                }
            }
        }
    }
}

internal val LIST_DETAIL_MIN_WIDTH = 840.dp
internal val LIST_PANE_WIDTH = 380.dp
internal val DETAIL_PANE_MAX_WIDTH = 720.dp

@Composable
private fun SettingsTab(group: SettingsGroup, everyPage: List<SettingsPageEntry>, modifier: Modifier, onOpenSetup: (String?) -> Unit, onOpen: (String) -> Unit) {
    val linksJson by qgcPath("view.links")
    val pages = remember(group, everyPage) { tabPages(group, everyPage) }
    Column(modifier.verticalScroll(rememberScrollState())) {
        if (group == SettingsGroup.General) AircraftSetupRow(onOpenSetup)
        pages.forEach { page ->
            key(page.title) {
                if (pageLook(page.title).inline || pageLook(page.title).group != group) {
                    InlinePage(page, group)
                } else {
                    SetupRow(
                        title = pageTitle(page.title),
                        status = if (page.title == CONNECTIONS_PAGE) activeLinksGlance(linksJson) else pageGlance(page.title),
                        icon = pageLook(page.title).icon,
                        onClick = { onOpen(page.title) },
                    )
                }
            }
        }
        TabSetupRows(group, onOpenSetup)
    }
}

@Composable
private fun InlinePage(page: SettingsPageEntry, group: SettingsGroup) {
    var sections by remember(page.title) { mutableStateOf<List<SettingsSectionRows>?>(null) }
    var reloads by remember(page.title) { mutableIntStateOf(0) }
    LaunchedEffect(page.title, reloads) {
        sections = withContext(Dispatchers.Default) { settingsSections(Qgc.get(settingsPagePath(page.title))) }
    }
    val shown = sections?.let { sectionsIn(group, page.title, it) }?.takeIf { it.isNotEmpty() } ?: return
    val home = pageLook(page.title).group == group
    if (!(home && pageLook(page.title).tabHome)) {
        PageHeader(pageLook(page.title).icon, if (home) pageTitle(page.title) else borrowedTitle(shown))
    }
    if (home && page.showsVideoSources) VideoPreview()
    SettingsControls(page, shown, home, blockHeadings = home) { reloads++ }
    if (home && page.title == GENERAL_PAGE) ResetAllSettingsRow()
}

internal fun borrowedTitle(sections: List<SettingsSectionRows>): String =
    sections.flatMap { it.blocks }.map { sentenceCase(it.title) }.filter { it.isNotBlank() }.distinct().joinToString(" \u00b7 ")

@Composable
private fun PageHeader(@DrawableRes icon: Int, title: String) {
    Row(
        Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 28.dp, bottom = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Icon(painterResource(icon), null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(24.dp))
        Text(title, style = MaterialTheme.typography.titleMedium)
    }
}

@Composable
internal fun SettingsSearch(query: String, modifier: Modifier = Modifier, onOpenSetup: (String?) -> Unit = {}, onOpen: (String) -> Unit) {
    val setupJson by qgcPath(SETUP)
    val setupHits = remember(setupJson, query) { setupSearchHits(setupComponents(setupJson), query) }
    var pages by remember { mutableStateOf(emptyList<SettingsPageEntry>()) }
    LaunchedEffect(Unit) {
        pages = withContext(Dispatchers.Default) { settingsPages(Qgc.get(SETTINGS_VIEW)) }
    }
    var hits by remember { mutableStateOf(emptyList<SettingsSectionRows>()) }
    val searching = query.isNotBlank()
    val pagePaths = remember(pages) { pages.map { settingsPagePath(it.title) } }
    DisposableEffect(pagePaths, searching) {
        if (searching) Qgc.watch(pagePaths)
        onDispose { if (searching) Qgc.unwatch(pagePaths) }
    }
    val values by Qgc.values.collectAsState()
    val served by remember(pagePaths) { derivedStateOf { pagePaths.map { values[it] } } }

    LaunchedEffect(query, pages, served) {
        if (!searching) {
            hits = emptyList()
            return@LaunchedEffect
        }
        delay(SEARCH_SETTLE_MS)
        hits = withContext(Dispatchers.Default) {
            pages.zip(served).flatMap { (page, json) ->
                matchesIn(page.title, settingsSections(json ?: Qgc.get(settingsPagePath(page.title))), query)
            }
        }
    }

    Box(modifier.fillMaxSize(), contentAlignment = Alignment.TopCenter) {
    LazyColumn(Modifier.fillMaxHeight().widthIn(max = DETAIL_PANE_MAX_WIDTH)) {
        if (!searching) return@LazyColumn

        val pageHits = pages.filter { pageMatches(it, query) }
        if (hits.isEmpty() && pageHits.isEmpty() && setupHits.isEmpty()) {
            item(key = "none") { FootNote("No settings match “${query.trim()}”.") }
            return@LazyColumn
        }

        if (setupHits.isNotEmpty()) item(key = "setupHead") { SectionHeader(AIRCRAFT_SETUP) }
        items(setupHits, key = { "setup${it.name}" }) { component ->
            SetupRow(title = sentenceCase(component.name), icon = setupIcon(component.known, component.className), onClick = { onOpenSetup(component.name) })
        }
        if (pageHits.isNotEmpty()) item(key = "pagesHead") { SectionHeader("Pages") }
        items(pageHits, key = { "page${it.title}" }) { entry ->
            SetupRow(title = pageTitle(entry.title), icon = pageLook(entry.title).icon, onClick = { onOpen(entry.title) })
        }

        hits.forEach { section ->
            item(key = "head${section.title}") { SectionHeader(shownBreadcrumb(section.title)) }
            items(section.blocks.flatMap { it.facts }, key = { it.path }) { fact ->
                FactRow(fact)
            }
        }
    }
    }
}

@Composable
private fun VideoPreview() {
    val previewMaxWidth = (LocalConfiguration.current.screenHeightDp * PREVIEW_HEIGHT_SHARE * 16f / 9f).dp
    Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
        VideoSurface(
            Modifier
                .padding(horizontal = 16.dp, vertical = 8.dp)
                .widthIn(max = previewMaxWidth)
                .fillMaxWidth()
                .aspectRatio(16f / 9f)
                .clip(MaterialTheme.shapes.large)
                .background(MaterialTheme.colorScheme.surfaceContainerHighest),
            expanded = true,
            settingsPreview = true,
        )
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
        if (page.showsVideoSources) VideoPreview()
        SettingsControls(page, sections) { reloads++ }
        if (page.title == GENERAL_PAGE) ResetAllSettingsRow()
    }
}

@Composable
private fun SettingsControls(
    page: SettingsPageEntry,
    sections: List<SettingsSectionRows>,
    home: Boolean = true,
    blockHeadings: Boolean = true,
    onWrite: () -> Unit,
) {
    if (home && page.showsNtrip) NtripStatusSection(onWrite)
    if (home && page.showsPacketRadio) PacketRadioSection(onWrite)
    sections.forEach { section ->
        if (section.group == UNITS_GROUP) {
            SectionHeader(section.title)
            UnitsSection()
            return@forEach
        }
        section.blocks.forEach { block ->
            if (block.title == ADVANCED_BLOCK) {
                AdvancedBlock("${section.group}#${block.title}") { FactRuns(block.facts, onWrite) }
                return@forEach
            }
            blockHeading(page.title, section, block).takeIf { blockHeadings && it.isNotBlank() }?.let { SectionHeader(sentenceCase(it)) }
            if (section.group == VIDEO_GROUP && block.title == CAMERAS_BLOCK && page.showsVideoSources) CamerasEditor()
            FactRuns(block.facts, onWrite)
            if (page.showsNtrip && block.title == NTRIP_MOUNTPOINT_BLOCK) NtripMountpointBrowser(onWrite)
            if (section.group == REMOTE_ID_GROUP && block.title == GCS_LOCATION_BLOCK) GcsPositionStatus()
            if (section.group == MAVLINK_GROUP && block.title == SIGNING_AFTER_BLOCK) SigningKeysSection()
        }
        if (!home) return@forEach
        section.note
            .takeIf { it.isNotBlank() && section.group !in GROUPS_WITH_A_HEAD_EDITOR }
            ?.let { FootNote(it) }
        if (section.group == FLY_VIEW_GROUP) RcControlsEditor()
        if (section.group == VIEWER_3D_GROUP) OsmFilePicker(onWrite)
        if (section.group == OFFLINE_MAPS_GROUP) OfflineMapsSection()
        if (section.group == MAVLINK_GROUP) LinkStatusSection()
        if (section.group == MAVLINK_ACTIONS_GROUP) MavlinkActionsSection(onWrite)
    }
}

internal const val ADVANCED_BLOCK = "Advanced"
internal const val CAMERAS_BLOCK = "Cameras"

@Composable
private fun AdvancedBlock(key: String, content: @Composable () -> Unit) {
    var open by rememberSaveable(key) { mutableStateOf(false) }
    Row(
        Modifier
            .fillMaxWidth()
            .toggleable(value = open, role = Role.Button) { open = it }
            .padding(start = 16.dp, end = 16.dp, top = 20.dp, bottom = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(ADVANCED_BLOCK, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.weight(1f))
        Icon(Icons.Filled.KeyboardArrowDown, contentDescription = if (open) "Hide advanced settings" else "Show advanced settings", tint = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.rotate(if (open) 180f else 0f))
    }
    if (open) content()
}

@Composable
internal fun FactRuns(facts: List<Fact>, onWrite: () -> Unit = {}) {
    val shared = sharedRebootNote(facts)
    shared?.let {
        Text(
            text = it,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.aircast.warning,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
        )
    }
    val inert = blockInertNote(facts)
    CompositionLocalProvider(LocalBlockRebootNote provides shared, LocalRunInertNote provides inert) { facts.forEach { FactRow(it, onWrite = onWrite) } }
    inert?.let {
        Text(
            text = it,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
        )
    }
}

internal val LocalBlockRebootNote = compositionLocalOf<String?> { null }

internal val LocalRunInertNote = compositionLocalOf<String?> { null }

internal val LocalDetailBehindHelp = compositionLocalOf { false }

internal const val SUBTITLE_SEPARATOR = " · "

internal data class ShownSubtitle(val text: String, val hasHelp: Boolean)

internal fun shownSubtitle(subtitle: String, detail: String, helpBehind: Boolean, helpOpen: Boolean): ShownSubtitle {
    val hasHelp = helpBehind && detail.isNotBlank() && (subtitle == detail || subtitle.startsWith(detail + SUBTITLE_SEPARATOR))
    val text = if (hasHelp && !helpOpen) subtitle.removePrefix(detail).removePrefix(SUBTITLE_SEPARATOR) else subtitle
    return ShownSubtitle(text, hasHelp)
}

@Composable
private fun FactTitle(title: String, color: Color, helpOpen: Boolean?, onHelp: () -> Unit, modifier: Modifier = Modifier, maxLines: Int = Int.MAX_VALUE) {
    Row(modifier, verticalAlignment = Alignment.CenterVertically) {
        Text(title, Modifier.weight(1f, fill = false), style = MaterialTheme.typography.bodyLarge, color = color, maxLines = maxLines, overflow = TextOverflow.Ellipsis)
        helpOpen?.let { open ->
            IconButton(onClick = onHelp) {
                Icon(
                    painterResource(R.drawable.ic_help),
                    if (open) "Hide help" else "Help",
                    tint = if (open) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.size(16.dp),
                )
            }
        }
    }
}

internal fun blockInertNote(facts: List<Fact>): String? =
    facts.filterNot { it.enabled }.takeIf { it.size > 1 }?.map(::inertNote)?.distinct()?.singleOrNull()

internal fun sharedRebootNote(facts: List<Fact>): String? =
    facts.mapNotNull(::factRebootNote).takeIf { it.size > 1 }?.distinct()?.singleOrNull()

internal const val NTRIP_MOUNTPOINT_BLOCK = "Mountpoint"

internal fun isSecret(fact: Fact): Boolean = fact.name.endsWith("Password", ignoreCase = true)

internal fun enumLabel(fact: Fact): String =
    fact.enumStrings.getOrNull(fact.enumIndex) ?: fact.valueString

internal fun shownEnumLabel(fact: Fact): String =
    fact.enumStrings.getOrNull(fact.enumIndex)?.let(::sentenceCase) ?: fact.valueString

internal fun factSubtitle(fact: Fact): String = when {
    fact.enumStrings.isNotEmpty() || fact.bitmaskStrings.isNotEmpty() || fact.isBool -> ""
    else -> fact.units
}

@Composable
internal fun FieldSlider(value: Float?, slider: FactSlider, enabled: Boolean, onWrite: (Double) -> Unit) {
    val held = value ?: slider.from
    var shown by remember(slider, held) { mutableStateOf(held.coerceIn(slider.from, slider.to)) }
    Slider(
        value = shown,
        onValueChange = { shown = it },
        valueRange = slider.from..slider.to,
        enabled = enabled,
        onValueChangeFinished = { onWrite(shown.toDouble()) },
    )
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Text(sliderValue(slider.from, slider.decimals, ""), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(sliderValue(slider.to, slider.decimals, ""), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
internal fun FactRow(
    fact: Fact,
    title: String = sentenceCase(fact.heading),
    subtitle: String = listOf(fact.detail, factSubtitle(fact)).filter { it.isNotBlank() }.joinToString(" · "),
    titleColor: Color = Color.Unspecified,
    fieldModifier: Modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
    onRejected: () -> Unit = {},
    onWrite: () -> Unit = {},
) {
    val scope = rememberCoroutineScope()
    var refusal by remember(fact.path) { mutableStateOf<String?>(null) }
    var helpOpen by remember(fact.path) { mutableStateOf(false) }
    val shown = shownSubtitle(subtitle, fact.detail, LocalDetailBehindHelp.current, helpOpen)
    val helpToggle: Boolean? = helpOpen.takeIf { shown.hasHelp }

    val segmented = !editOnDesktop(fact) && !fact.isBitmask &&
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
            val runInert = LocalRunInertNote.current
            val note = (shown.text.split(SUBTITLE_SEPARATOR) + listOfNotNull(inertNote(fact).takeIf { !fact.enabled && it != runInert }))
                .filter { it.isNotBlank() && it != fact.units }.joinToString(" · ")
            if (valueOnTheRight(fact)) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                    Column(Modifier.weight(1f)) {
                        FactTitle(title, titleColor, helpToggle, { helpOpen = !helpOpen })
                        if (note.isNotBlank()) Text(note, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    if (fact.isEnum && !fact.valueIsOffTheEnumList) {
                        EnumField(fact, Modifier.width(CHOICE_VALUE_WIDTH), ::write)
                    } else {
                        FactTextField(fact, onWrite, onRejected, onTheRight = true)
                    }
                }
            } else {
                FactTitle(title, titleColor, helpToggle, { helpOpen = !helpOpen }, Modifier.padding(bottom = 8.dp))
                if (fact.isBitmask) BitmaskPicker(fact, ::write) else FactTextField(fact, onWrite, onRejected)
            }
            fact.slider?.takeIf { !fact.isEnum && !fact.isBitmask }?.let { slider -> FieldSlider((fact.value as? Number)?.toFloat() ?: fact.valueString.toFloatOrNull(), slider, fact.acceptsWrite) { value -> write { Qgc.set(fact.path, value) } } }
            if (note.isNotBlank() && !valueOnTheRight(fact)) {
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

    val rowToggles = !segmented && fact.isBool && fact.acceptsWrite && !editOnDesktop(fact)
    val rows: @Composable (Boolean) -> Unit = { beside ->
    Column {
    Row(
        Modifier
            .fillMaxWidth()
            .then(if (rowToggles) Modifier.toggleable(value = fact.boolValue, role = Role.Switch) { checked -> write { Qgc.set(fact.path, checked != fact.inverted) } } else Modifier)
            .heightIn(min = 64.dp)
            .padding(horizontal = 16.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Column(Modifier.weight(1f)) {
            FactTitle(title, titleColor, helpToggle, { helpOpen = !helpOpen }, maxLines = 3)
            val rowNote = shown.text.split(SUBTITLE_SEPARATOR).filter { it.isNotBlank() && it != fact.units }.joinToString(" · ")
            if (rowNote.isNotBlank() && !rowNote.equals(title, ignoreCase = true)) {
                Text(
                    text = rowNote,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            if (!fact.acceptsWrite && !editOnDesktop(fact)) {
                Text(
                    text = inertNote(fact),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }

        if (beside) Segments(fact, ::write)
        if (!segmented) Box(Modifier.widthIn(max = 190.dp), contentAlignment = Alignment.CenterEnd) {
            when {
                editOnDesktop(fact) -> Column(
                    horizontalAlignment = Alignment.End,
                ) {
                    Text(
                        text = shownEnumLabel(fact),
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
                    if (fact.isBool) {
                        Switch(checked = fact.boolValue, onCheckedChange = null, enabled = false)
                    } else {
                        Text(
                            text = listOf(shownEnumLabel(fact), fact.units.takeIf { !fact.isEnum }.orEmpty()).filter { it.isNotBlank() }.joinToString(" "),
                            style = MaterialTheme.typography.bodyMedium,
                            maxLines = 2,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
                fact.isBool -> Switch(checked = fact.boolValue, onCheckedChange = null)
                else -> BitmaskPicker(fact, ::write)
            }
        }
    }
    if (segmented && !beside) Segments(fact, ::write, Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, bottom = 10.dp))
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
    if (segmented) BoxWithConstraints { rows(maxWidth >= SEGMENTS_BESIDE_MIN_WIDTH) } else rows(false)
}

@Composable
private fun Segments(fact: Fact, write: (() -> Boolean) -> Unit, modifier: Modifier = Modifier) {
    SingleChoiceSegmentedButtonRow(modifier) {
        fact.enumStrings.forEachIndexed { index, option ->
            val boolOption = fact.enumValues.getOrNull(index)?.toBooleanStrictOrNull()?.takeIf { fact.isBool }
            SegmentedButton(
                selected = boolOption?.let { it == fact.boolValue } ?: (index == fact.enumIndex),
                onClick = { write { boolOption?.let { Qgc.set(fact.path, it != fact.inverted) } ?: Qgc.set("${fact.path}.enumIndex", index) } },
                shape = SegmentedButtonDefaults.itemShape(index, fact.enumStrings.size),
                icon = {},
            ) { Text(sentenceCase(option), maxLines = 1) }
        }
    }
}

internal const val SEGMENT_LABEL_BUDGET = 28
internal val SEGMENTS_BESIDE_MIN_WIDTH = 560.dp
internal val NUMBER_VALUE_WIDTH = 160.dp
internal val CHOICE_VALUE_WIDTH = 220.dp

internal fun sentenceCase(label: String): String = one.aircast.map.sentenceCase(label)

private val SECONDS = setOf("s", "sec", "secs", "second", "seconds")

internal fun shownUnits(units: String): String = if (units.lowercase() in SECONDS) "s" else units

internal fun showsAsField(fact: Fact): Boolean =
    !fact.readOnly && !editOnDesktop(fact) && !fact.isBool

internal fun valueOnTheRight(fact: Fact): Boolean = !fact.isString && !fact.isBitmask

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
private fun BitmaskPicker(fact: Fact, write: (() -> Boolean) -> Unit) {
    var editing by remember(fact.path) { mutableStateOf(false) }
    val raw = bitmaskRaw(fact)

    Box {
        OutlinedTextField(
            value = bitmaskSummary(fact),
            onValueChange = {},
            readOnly = true,
            maxLines = 3,
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
                        val live = fact.enabled && bitmaskEntryEnabled(fact, raw, index)
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
private fun EnumField(fact: Fact, modifier: Modifier, write: (() -> Boolean) -> Unit) {
    ChoiceField(null, shownEnumLabel(fact), fact.enumStrings.map(::sentenceCase), modifier, enabled = fact.enabled, groups = fact.enumGroups) { index ->
        // qtpaths: settings.appSettings.indoorPalette.enumIndex, vehicle.parameterManager.getParameter(-1,RTL_TYPE).enumIndex
        write { fact.enumValues.getOrNull(index)?.toLongOrNull()?.takeIf { fact.rawChoice }?.let { Qgc.set(fact.path, it) } ?: Qgc.set("${fact.path}.enumIndex", index) }
    }
}

@Composable
internal fun ChoiceField(label: String?, value: String, options: List<String>, modifier: Modifier = Modifier, enabled: Boolean = true, groups: List<String> = emptyList(), onPick: (Int) -> Unit) {
    var expanded by remember { mutableStateOf(false) }

    Box(modifier) {
        OutlinedTextField(
            value = value,
            onValueChange = {},
            readOnly = true,
            enabled = enabled,
            singleLine = true,
            label = label?.let { { Text(it, maxLines = 1, overflow = TextOverflow.Ellipsis) } },
            trailingIcon = { Icon(Icons.Default.KeyboardArrowDown, contentDescription = null) },
            modifier = Modifier.fillMaxWidth(),
        )
        Box(Modifier.matchParentSize().clickable(enabled = enabled) { expanded = true })
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            options.forEachIndexed { index, option ->
                groups.getOrNull(index)?.takeIf { it.isNotBlank() && it != groups.getOrNull(index - 1) }?.let { group ->
                    if (index > 0) androidx.compose.material3.HorizontalDivider()
                    Text(group, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary, modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp))
                }
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
    return if (typed == floor(typed)) null else "Invalid number"
}

internal fun typedValue(text: String): String = text.replace("\n", "")

internal const val ASPECT_RATIO = "aspectRatio"
private const val RATIO_TOLERANCE = 0.005
private val COMMON_RATIOS = listOf(16 to 9, 4 to 3, 21 to 9, 16 to 10, 3 to 2, 5 to 4, 1 to 1)

internal fun ratioText(value: Double): String? =
    COMMON_RATIOS.firstOrNull { (width, height) -> abs(width.toDouble() / height - value) < RATIO_TOLERANCE }?.let { (width, height) -> "$width:$height" }

internal fun ratioValue(text: String): String? =
    text.split(':').takeIf { it.size == 2 }?.map { it.trim().toDoubleOrNull() }?.let { (width, height) ->
        if (width != null && height != null && width > 0.0 && height > 0.0) String.format(java.util.Locale.ROOT, "%.6f", width / height) else null
    }

internal fun isAddress(fact: Fact): Boolean = fact.isString && (fact.name.endsWith("Url") || fact.name.endsWith("URL"))

internal fun factKeyboard(fact: Fact): KeyboardType = when {
    isAddress(fact) -> KeyboardType.Uri
    fact.isString || fact.isBool -> KeyboardType.Text
    fact.wholeNumbersOnly && fact.minString.toDoubleOrNull()?.let { it >= 0.0 } == true ->
        KeyboardType.Number
    fact.minString.toDoubleOrNull()?.let { it < 0.0 } != false -> KeyboardType.Text
    else -> KeyboardType.Decimal
}

internal fun fieldText(fact: Fact): String =
    if (fact.isString || isSecret(fact)) fact.valueString else plainNumber(fact.valueString)

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

private fun blockingRejection(fact: Fact, text: String): String? =
    truncationRefusal(fact, text) ?:
        // qtpaths: settings.appSettings.indoorPalette.validate, vehicle.parameterManager.getParameter(-1,RTL_ALT).validate
        validationMessage(Qgc.invokeResult("${fact.path}.validate", text, false))

private suspend fun rejectionFor(fact: Fact, text: String): String? =
    withContext(Dispatchers.Default) { blockingRejection(fact, text) }

internal fun storedText(fact: Fact, typed: String): String =
    if (fact.name == ASPECT_RATIO) ratioValue(typed) ?: typed else typed

internal fun shownText(fact: Fact): String =
    if (fact.name == ASPECT_RATIO) fact.valueString.toDoubleOrNull()?.let(::ratioText) ?: fieldText(fact) else fieldText(fact)

@Composable
private fun FactTextField(fact: Fact, onWrite: () -> Unit, onRejected: () -> Unit = {}, onTheRight: Boolean = false) {
    var editing by remember(fact.path) { mutableStateOf<String?>(null) }
    var rejection by remember(fact.path) { mutableStateOf<String?>(null) }
    var revealed by remember(fact.path) { mutableStateOf(false) }
    val secret = isSecret(fact)
    val scope = rememberCoroutineScope()
    val focusManager = LocalFocusManager.current

    val interaction = remember { androidx.compose.foundation.interaction.MutableInteractionSource() }
    val focused by interaction.collectIsFocusedAsState()
    val pending = editing?.takeIf { it != shownText(fact) }
    val commit: () -> Unit = {
        pending?.let { typed ->
            val committed = storedText(fact, typed)
            scope.launch {
                val refused = rejectionFor(fact, committed)
                if (refused != null) {
                    rejection = refused
                    onRejected()
                    return@launch
                }
                val refusal = withContext(Dispatchers.Default) {
                    Qgc.writeRefusal(fact.path, committed)
                }
                rejection = refusal
                if (refusal == null) {
                    editing = null
                    onWrite()
                }
            }
        }
    }
    val latestCommit by rememberUpdatedState(commit)
    val latestPending by rememberUpdatedState(pending)
    LaunchedEffect(interaction) {
        snapshotFlow { focused }.drop(1).filter { !it }.collect { latestCommit() }
    }
    DisposableEffect(fact.path) {
        onDispose {
            latestPending?.let { typed ->
                val committed = storedText(fact, typed)
                offMainDetached { if (blockingRejection(fact, committed) == null) Qgc.writeRefusal(fact.path, committed) }
            }
        }
    }
    Column(if (onTheRight) Modifier.width(NUMBER_VALUE_WIDTH) else Modifier) {
        OutlinedTextField(
            value = editing ?: shownText(fact),
            enabled = fact.enabled,
            textStyle = if (onTheRight) LocalTextStyle.current.copy(textAlign = TextAlign.End) else LocalTextStyle.current,
            suffix = fact.units.takeIf { it.isNotBlank() }?.let { { Text(shownUnits(it)) } },
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
            keyboardOptions = KeyboardOptions(keyboardType = factKeyboard(fact), autoCorrectEnabled = !isAddress(fact), imeAction = ImeAction.Done),
            keyboardActions = KeyboardActions(onDone = { focusManager.clearFocus() }),
            modifier = Modifier.fillMaxWidth(),
            interactionSource = interaction,
            trailingIcon = pending?.let { { TextButton(onClick = commit) { Text("Set") } } },
        )
        rejection?.let {
            Text(
                text = it,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(start = 16.dp, top = 4.dp),
            )
        }
        if (rejection == null && fact.problem.isNotBlank() && editing.isNullOrBlank()) {
            Text(
                text = fact.problem,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(start = 16.dp, top = 4.dp),
            )
        }
        if (rejection == null && focused) {
            factConstraintNote(fact)?.let {
                Text(
                    text = it,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(start = 16.dp, top = 4.dp),
                )
            }
        }
        factRebootNote(fact)?.takeIf { it != LocalBlockRebootNote.current }?.let {
            Text(
                text = it,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.aircast.warning,
                modifier = Modifier.padding(start = 16.dp, top = 4.dp),
            )
        }
    }
}

package one.aircast.android.ui

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.snapshotFlow
import kotlinx.coroutines.flow.filter
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.key
import androidx.compose.runtime.remember
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import kotlinx.coroutines.launch
import one.aircast.android.R
import one.aircast.map.AircastSpace
import one.aircast.map.aircast

internal fun sheetDrilled(openPage: String?, setupOpen: Boolean): Boolean = openPage != null || setupOpen

internal fun openingGroup(requested: String?): SettingsGroup = requested?.let { pageLook(it).group } ?: SettingsGroup.Safety

@Composable
internal fun SettingsSheet(requested: String?, onClose: () -> Unit) {
    val navigation = LocalAppNavigation.current
    var group by rememberSaveable(requested) { mutableStateOf(if (navigation.aircraftRequested) SettingsGroup.General else openingGroup(requested)) }
    var setupOpen by rememberSaveable { mutableStateOf(navigation.aircraftRequested) }
    var setupFromTab by rememberSaveable { mutableStateOf(false) }
    var enteredForSetup by rememberSaveable { mutableStateOf(navigation.aircraftRequested) }
    LaunchedEffect(setupOpen) { if (!setupOpen) enteredForSetup = false }
    var page by rememberSaveable(requested) { mutableStateOf(requested) }
    var query by rememberSaveable { mutableStateOf<String?>(null) }
    var returnQuery by rememberSaveable { mutableStateOf<String?>(null) }
    val backToSearch: () -> Unit = {
        query = returnQuery
        returnQuery = null
        page = null
    }
    val closeSetup: () -> Unit = {
        if (enteredForSetup) {
            onClose()
        } else {
            setupOpen = false
            if (returnQuery != null) backToSearch()
        }
    }
    LaunchedEffect(navigation.aircraftRequested) {
        if (navigation.aircraftRequested) {
            setupFromTab = setupFromTab || !navigation.setupPage.isNullOrEmpty()
            setupOpen = true
            query = null
            navigation.aircraftRequested = false
        }
    }
    LaunchedEffect(navigation.settingsPage) {
        navigation.settingsPage?.let { asked ->
            group = pageLook(asked).group
            page = asked
            setupOpen = false
            query = null
            navigation.settingsPage = null
        }
    }
    androidx.compose.runtime.DisposableEffect(group, setupOpen) {
        navigation.settingsShowing = group.takeUnless { setupOpen }
        onDispose { navigation.settingsShowing = null }
    }
    androidx.activity.compose.BackHandler(onBack = onClose)
    androidx.activity.compose.BackHandler(enabled = query != null) { query = null }
    androidx.activity.compose.BackHandler(enabled = setupOpen && query == null, onBack = closeSetup)
    val initialPage = page?.takeIf { pageLook(it).group == group }
    var openPage by rememberSaveable(group, page, query != null, setupOpen) { mutableStateOf(initialPage?.takeUnless { pageLook(it).inline }) }
    val closePage: () -> Unit = { backToSearch.takeIf { returnQuery != null && openPage == initialPage }?.invoke() ?: run { openPage = null } }
    val openSetup: (String?) -> Unit = { component ->
        navigation.setupPage = component
        setupFromTab = component != null
        setupOpen = true
        returnQuery = query?.takeIf { it.isNotBlank() }
        query = null
    }
    val snackbars = remember { androidx.compose.material3.SnackbarHostState() }
    val noticeScope = androidx.compose.runtime.rememberCoroutineScope()
    val notice = remember {
        ChangeNotice { message, undo ->
            noticeScope.launch {
                snackbars.currentSnackbarData?.dismiss()
                val result = snackbars.showSnackbar(message, actionLabel = undo?.let { "Undo" }, duration = androidx.compose.material3.SnackbarDuration.Long)
                if (result == androidx.compose.material3.SnackbarResult.ActionPerformed) undo?.invoke()?.let { refusal -> snackbars.showSnackbar("Undo failed: $refusal") }
            }
        }
    }
    Surface(Modifier.fillMaxSize().zIndex(SETTINGS_SHEET_LAYER).pointerInput(Unit) {}, color = MaterialTheme.colorScheme.surface.copy(alpha = if (LocalConfiguration.current.orientation == android.content.res.Configuration.ORIENTATION_LANDSCAPE) SETTINGS_PANEL_ALPHA else 1f)) {
      CompositionLocalProvider(LocalChangeNotice provides notice) {
      Box(Modifier.fillMaxSize()) {
        Column(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing)) {
            val pickTab: (SettingsGroup) -> Unit = { entry ->
                group = entry
                page = null
                setupOpen = false
                returnQuery = null
            }
            BoxWithConstraints {
                val stacked = maxWidth < STACKED_HEADER_WIDTH
                val drilled = sheetDrilled(openPage, setupOpen)
                Column {
                    Row(
                        Modifier.fillMaxWidth().padding(horizontal = AircastSpace.s3, vertical = AircastSpace.s1),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                    ) {
                        query?.let { typed ->
                            val focus = remember { FocusRequester() }
                            LaunchedEffect(Unit) { focus.requestFocus() }
                            SearchPill(typed, { query = it }, "Search settings", Modifier.weight(1f).focusRequester(focus))
                            TextButton(onClick = { query = null }) { Text("Cancel") }
                        } ?: run {
                            if (stacked || drilled) {
                                Text("Settings", style = MaterialTheme.typography.titleLarge, modifier = Modifier.weight(1f).padding(start = AircastSpace.s2))
                            } else {
                                SheetTabs(group.takeUnless { setupOpen }, Modifier.weight(1f), pickTab)
                            }
                            IconButton(onClick = { query = "" }) { Icon(painterResource(R.drawable.ic_search), "Search settings") }
                        }
                        IconButton(onClick = onClose) { Icon(painterResource(R.drawable.ic_close), "Close settings") }
                    }
                    if (stacked && query == null && !drilled) SheetTabs(group.takeUnless { setupOpen }, Modifier.fillMaxWidth().padding(horizontal = AircastSpace.s2), pickTab)
                }
            }
            if (query == null && !setupOpen) {
                val everyPage = rememberSettingsPages()
                SettingsPager(group, swipeable = openPage == null, onSettled = pickTab, modifier = Modifier.weight(1f)) { shown ->
                    val current = shown == group
                    key(page.takeIf { current }) {
                        SettingsScreen(shown, everyPage, openPage.takeIf { current }, { openPage = it }, closePage, Modifier.fillMaxSize(), onOpenSetup = openSetup)
                    }
                }
            } else key(group, page, query != null, setupOpen) {
                when {
                    query != null -> SettingsSearch(query.orEmpty(), Modifier.weight(1f), openSetup) { title ->
                        group = pageLook(title).group
                        page = title
                        returnQuery = query.takeUnless { pageLook(title).inline }
                        query = null
                        setupOpen = false
                    }
                    else -> Column(Modifier.weight(1f)) {
                        val heading = remember { mutableStateOf<PageHeading?>(null) }
                        val back: () -> Unit = { heading.value?.takeUnless { setupFromTab }?.back?.invoke() ?: closeSetup() }
                        PageTopBar(heading.value?.title ?: AIRCRAFT_SETUP, "Back", back)
                        CompositionLocalProvider(LocalPageHeading provides heading) { SetupScreen(Modifier.weight(1f)) }
                        androidx.activity.compose.BackHandler(enabled = setupFromTab && query == null, onBack = back)
                    }
                }
            }
        }
        androidx.compose.material3.SnackbarHost(snackbars, Modifier.align(Alignment.BottomCenter).windowInsetsPadding(WindowInsets.safeDrawing)) { AppSnackbar(it) }
      }
      }
    }
}

@Composable
private fun SettingsPager(group: SettingsGroup, swipeable: Boolean, onSettled: (SettingsGroup) -> Unit, modifier: Modifier, content: @Composable (SettingsGroup) -> Unit) {
    val groups = SettingsGroup.entries
    val pager = rememberPagerState(initialPage = group.ordinal) { groups.size }
    val shownGroup by rememberUpdatedState(group)
    val settled by rememberUpdatedState(onSettled)
    LaunchedEffect(group) { if (pager.currentPage != group.ordinal) pager.scrollToPage(group.ordinal) }
    LaunchedEffect(pager) {
        snapshotFlow { groups[pager.settledPage] }.filter { it != shownGroup }.collect { settled(it) }
    }
    HorizontalPager(pager, modifier, userScrollEnabled = swipeable, beyondViewportPageCount = 1, key = { groups[it].name }) { index -> content(groups[index]) }
}

@Composable
private fun SheetTabs(selected: SettingsGroup?, modifier: Modifier, onPick: (SettingsGroup) -> Unit) {
    Row(modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(AircastSpace.s1)) {
        SettingsGroup.entries.map { entry -> SheetTab(entry.title, entry == selected) { onPick(entry) } }
    }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun SheetTab(title: String, selected: Boolean, onClick: () -> Unit) {
    val shown = remember { BringIntoViewRequester() }
    LaunchedEffect(selected) { if (selected) shown.bringIntoView() }
    Column(
        Modifier
            .bringIntoViewRequester(shown)
            .selectable(selected = selected, role = Role.Tab, onClick = onClick)
            .heightIn(min = 48.dp)
            .padding(horizontal = AircastSpace.s2, vertical = AircastSpace.s1),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Text(
            title,
            style = MaterialTheme.typography.titleMedium,
            fontWeight = if (selected) FontWeight.SemiBold else FontWeight.Normal,
            color = if (selected) MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Box(
            Modifier
                .padding(top = 4.dp)
                .size(width = 20.dp, height = 3.dp)
                .background(if (selected) MaterialTheme.colorScheme.primary else Color.Transparent, CircleShape),
        )
    }
}

private val STACKED_HEADER_WIDTH = 600.dp

private const val SETTINGS_SHEET_LAYER = 10f
private const val SETTINGS_PANEL_ALPHA = 0.9f

@Composable
internal fun FlySettingsButton() {
    val navigation = LocalAppNavigation.current
    IconButton(onClick = { navigation.settingsOpen = true }, modifier = Modifier.semantics { contentDescription = "Settings" }) {
        Text("⋯", style = MaterialTheme.typography.titleLarge, color = MaterialTheme.aircast.outdoorForeground, modifier = Modifier.osdShadow())
    }
}

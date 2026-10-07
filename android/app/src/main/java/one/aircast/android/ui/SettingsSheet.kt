package one.aircast.android.ui

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.zIndex
import one.aircast.android.R
import one.aircast.map.AircastSpace
import one.aircast.map.aircast

internal fun openingGroup(requested: String?): SettingsGroup = requested?.let { pageLook(it).group } ?: SettingsGroup.Safety

@Composable
internal fun SettingsSheet(requested: String?, onClose: () -> Unit) {
    var group by rememberSaveable(requested) { mutableStateOf(openingGroup(requested)) }
    val initialPage = remember(requested) { requested }
    androidx.activity.compose.BackHandler(onBack = onClose)
    Surface(Modifier.fillMaxSize().zIndex(SETTINGS_SHEET_LAYER).pointerInput(Unit) {}, color = MaterialTheme.colorScheme.surface) {
            Column(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing)) {
                Row(
                    Modifier.fillMaxWidth().padding(horizontal = AircastSpace.s3, vertical = AircastSpace.s2),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2),
                ) {
                    Row(Modifier.weight(1f).horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2)) {
                        SettingsGroup.entries.map { entry ->
                            FilterChip(selected = entry == group, onClick = { group = entry }, label = { Text(entry.title) })
                        }
                    }
                    StatusPill()
                    IconButton(onClick = onClose) { Icon(painterResource(R.drawable.ic_close), "Close settings") }
                }
                key(group) { SettingsScreen(group, initialPage.takeIf { group == openingGroup(requested) }, Modifier.weight(1f)) }
            }
    }
}

private const val SETTINGS_SHEET_LAYER = 10f

@Composable
internal fun FlySettingsButton() {
    val navigation = LocalAppNavigation.current
    IconButton(onClick = { navigation.settingsOpen = true }, modifier = Modifier.semantics { contentDescription = "Settings" }) {
        Text("\u22EF", style = MaterialTheme.typography.titleLarge, color = MaterialTheme.aircast.outdoorForeground, modifier = Modifier.osdShadow())
    }
}

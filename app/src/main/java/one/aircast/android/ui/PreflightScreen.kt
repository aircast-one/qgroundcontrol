package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.Checkbox
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.qgcPath

@Composable
fun PreflightScreen(
    modifier: Modifier = Modifier,
    ticked: Set<String> = emptySet(),
    onTicked: (Set<String>) -> Unit = {},
) {
    val json by qgcPath(PREFLIGHT)
    val checks = remember(json) { preflight(json) }

    if (checks == null || checks.groups.isEmpty()) {
        Text(
            "Connect a vehicle to run its preflight checks.",
            modifier.fillMaxWidth().padding(16.dp),
        )
        return
    }

    Column(modifier.fillMaxSize()) {
        Text(
            text = preflightSummary(checks, ticked),
            style = MaterialTheme.typography.titleSmall,
            color = if (checks.blocked.isEmpty()) {
                MaterialTheme.colorScheme.onSurface
            } else {
                MaterialTheme.colorScheme.error
            },
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
        )
        if (checks.airframe.isNotBlank()) {
            Text(
                text = checks.airframe,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(horizontal = 16.dp),
            )
        }
        HorizontalDivider(Modifier.padding(top = 8.dp))

        LazyColumn(Modifier.fillMaxSize()) {
            checks.groups.forEach { group ->
                item(key = "group:${group.name}") {
                    Text(
                        text = group.name,
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.padding(start = 16.dp, top = 16.dp, bottom = 4.dp),
                    )
                }
                items(group.checks.size, key = { "${group.name}:${group.checks[it].name}" }) { index ->
                    val check = group.checks[index]
                    val isTicked = check.name in ticked
                    Row(
                        Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 12.dp, vertical = 6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        Checkbox(
                            checked = isTicked || check.verdict == "passing",
                            enabled = checkNeedsTicking(check),
                            onCheckedChange = { on ->
                                onTicked(if (on) ticked + check.name else ticked - check.name)
                            },
                        )
                        Column(Modifier.weight(1f)) {
                            Text(
                                text = check.prompt.ifBlank { check.name },
                                style = MaterialTheme.typography.bodyMedium,
                            )
                            Text(
                                text = checkStatusText(check, isTicked),
                                style = MaterialTheme.typography.labelSmall,
                                color = if (check.blocked) {
                                    MaterialTheme.colorScheme.error
                                } else {
                                    MaterialTheme.colorScheme.onSurfaceVariant
                                },
                            )
                        }
                    }
                }
            }
        }
    }
}

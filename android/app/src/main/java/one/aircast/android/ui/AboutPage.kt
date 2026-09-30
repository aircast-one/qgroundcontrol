package one.aircast.android.ui

import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.foundation.clickable
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import one.aircast.android.BuildConfig

@Composable
internal fun AboutPage(links: List<HelpLink>, modifier: Modifier = Modifier) {
    val context = LocalContext.current
    Column(modifier.padding(vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        SectionHeader("About")
        ListItem(
            headlineContent = { Text("Aircast Version") },
            trailingContent = { Text(BuildConfig.VERSION_NAME, style = MaterialTheme.typography.bodyMedium) },
        )
        SectionHeader("Support")
        links.forEach { link ->
            ListItem(
                headlineContent = { Text(link.name) },
                supportingContent = { Text(link.host, color = MaterialTheme.colorScheme.primary) },
                modifier = Modifier.clickable {
                    context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(link.url)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
                },
            )
        }
    }
}

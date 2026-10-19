package one.aircast.android.ui

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import one.aircast.android.R
import one.aircast.android.bridge.qgcDerived

private const val PREVIEW_ASPECT = 16f / 9f
private val PREVIEW_SHAPE = RoundedCornerShape(12.dp)

@Composable
internal fun WhepPreview(endpoint: String, watch: String, name: String) {
    val uris = LocalUriHandler.current
    val preview by qgcDerived(videoPreviewPath(endpoint), ::videoPreview)
    Box(
        Modifier.fillMaxWidth().aspectRatio(PREVIEW_ASPECT).clip(PREVIEW_SHAPE).background(Color.Black).clickable(onClickLabel = "Open the $name camera") { runCatching { uris.openUri(watch) } },
        contentAlignment = Alignment.Center,
    ) {
        preview?.channel?.let { channel -> key(channel) { VideoChannelSurface(channel, Modifier.fillMaxSize()) } }
        if (preview?.playing != true) {
            Column(
                Modifier.matchParentSize().background(Color.Black).padding(horizontal = 24.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterVertically),
            ) {
                if (preview?.trying != false) CircularProgressIndicator(Modifier.size(32.dp), color = Color.White)
                Text(preview?.text.orEmpty(), style = MaterialTheme.typography.labelMedium, color = Color.White, textAlign = TextAlign.Center)
            }
        }
        Row(
            Modifier.align(Alignment.BottomEnd).padding(8.dp).background(Color.Black.copy(alpha = 0.6f), RoundedCornerShape(4.dp)).padding(horizontal = 8.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            Icon(painterResource(R.drawable.ic_open_in_new), null, tint = Color.White, modifier = Modifier.size(14.dp))
            Text("Open camera", style = MaterialTheme.typography.labelSmall, color = Color.White)
        }
    }
}

@Composable
internal fun PreviewPlaceholder(text: String) {
    Box(Modifier.fillMaxWidth().aspectRatio(PREVIEW_ASPECT).background(Color.Black.copy(alpha = 0.8f), PREVIEW_SHAPE).padding(horizontal = 24.dp), contentAlignment = Alignment.Center) {
        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Icon(painterResource(R.drawable.ic_videocam_off), null, tint = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.size(28.dp))
            Text(text, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, textAlign = TextAlign.Center)
        }
    }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
internal fun PartDisclosure(part: SetupPart, id: String, opened: String, content: @Composable () -> Unit) {
    val head = remember { BringIntoViewRequester() }
    LaunchedEffect(Unit) { if (opened == id) head.bringIntoView() }
    Column(Modifier.fillMaxWidth().bringIntoViewRequester(head).semantics { contentDescription = part.title }) {
        DeviceDisclosure(part.title, part.summary, part.badge, initiallyOpen = part.open || opened == id) {
            Column(Modifier.fillMaxWidth().padding(bottom = 8.dp)) { content() }
        }
    }
}

@Composable
internal fun LinkedNoticeView(notice: LinkedNotice, onOpen: (String) -> Unit) {
    if (notice.text.isEmpty()) return
    DeviceNotice(DeviceBadge(notice.level, notice.text))
    notice.link?.let { link -> TextButton(onClick = { onOpen(link.href) }, modifier = Modifier.padding(horizontal = 8.dp)) { Text("${link.label} →") } }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun NoteLink(text: String, label: String, style: TextStyle = MaterialTheme.typography.bodySmall, onClick: () -> Unit) {
    FlowRow(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(text, style = style, color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.align(Alignment.CenterVertically))
        TextButton(onClick = onClick) { Text(label) }
    }
}

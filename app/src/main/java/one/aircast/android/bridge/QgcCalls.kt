package one.aircast.android.bridge

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

fun CoroutineScope.offMain(block: () -> Unit) {
    launch(Dispatchers.Default) { block() }
}

fun offMainDetached(block: () -> Unit) {
    Thread(block).start()
}

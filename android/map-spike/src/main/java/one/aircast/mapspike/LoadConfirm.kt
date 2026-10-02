package one.aircast.mapspike

enum class LoadStep { Confirm, Load }

fun loadStep(dirty: Boolean, armed: Boolean): LoadStep =
    if (dirty && !armed) LoadStep.Confirm else LoadStep.Load

fun replaceWarning(items: Int): String =
    "Your unsaved plan here (${itemCountText(items)}) will be replaced."

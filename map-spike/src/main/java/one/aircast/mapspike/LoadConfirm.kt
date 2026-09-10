package one.aircast.mapspike

enum class LoadStep { Confirm, Load }

fun loadStep(dirty: Boolean, containsItems: Boolean, armed: Boolean): LoadStep =
    if (dirty && containsItems && !armed) LoadStep.Confirm else LoadStep.Load

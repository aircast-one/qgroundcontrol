package one.aircast.mapspike

enum class LoadStep { Confirm, Load }

// Loading from the vehicle overwrites whatever is drawn, and there is no undo.
// QGC asks first when the plan has unsent changes; a map that quietly discarded
// them would be the more dangerous of the two.
fun loadStep(dirty: Boolean, armed: Boolean): LoadStep =
    if (dirty && !armed) LoadStep.Confirm else LoadStep.Load

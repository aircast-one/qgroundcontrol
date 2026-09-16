# The Android flight head

## Why three dialogs remain on the flight surface

Takeoff height, change altitude and change speed each used to open a full-screen
dialog. They no longer do (`e3ee0d221`): the readings an operator picks a number
from — altitude, distance to home, ground speed — were behind the dialog at the
moment of the decision, so all three are now a panel with the commit at the
slider's foot and the telemetry still on screen.

Three `AlertDialog`s in `VehicleUi.kt` were deliberately left alone: the Actions
sheet, the pre-flight checklist and the mode-switch confirmation.

**The rule, which is not the same as "remove every dialog":** a dialog is right
when the operator *should* stop looking at the readings, and wrong when the
readings are the input. A checklist and a mode switch want full attention. A
height chosen against a live altitude does not.

Anyone tidying the remaining three would be completing a job that was finished.

## What the emergency stop is allowed to depend on

`PinnedEmergencyStop` reads `view.guidedActions` itself and carries its own
confirm, so it can be drawn outside the action bar. That is not tidiness: the
bar sits inside an `AnimatedVisibility`, and while the stop lived there,
collapsing the flight controls took it off screen (`878c94e22`, fixed in
`dece2e9e7`).

It is drawn when the core says `blocked`, with the served reason beneath it,
rather than hidden — hiding the control at the moment an operator reaches for it
is the worse failure. Its confirmation is forced destructive whatever the served
flag says, because a head's safety claim should not depend on a field another
layer can change.

There is one call site per orientation and no branch between the orientation
gate and the call. Keep it that way; it is what makes "present in every state"
checkable rather than asserted.

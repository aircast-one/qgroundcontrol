#include "QGCEntry.h"

#include "QGCCorePlugin.h"

void qgc_set_host_provides_ui(int provides)
{
    QGCCorePlugin::setHostProvidesUI(provides != 0);
}

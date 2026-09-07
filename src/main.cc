/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "QGCEntry.h"

#ifdef QGC_NATIVE_UI
#include "QGCNativeUI.h"

int main(int argc, char *argv[])
{
    return qgc_macos_main(argc, argv);
}
#else
int main(int argc, char *argv[])
{
    const int startCode = qgc_start(argc, argv);
    if (startCode != 0) {
        return startCode;
    }
    const int exitCode = qgc_run();
    qgc_shutdown();
    return exitCode;
}
#endif

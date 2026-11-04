#include "QGCEntry.h"

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

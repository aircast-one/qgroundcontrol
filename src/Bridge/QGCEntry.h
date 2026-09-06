#pragma once

#ifdef __cplusplus
extern "C" {
#endif

int qgc_start(int argc, char *argv[]);
int qgc_run(void);
void qgc_shutdown(void);

#ifdef __cplusplus
}
#endif

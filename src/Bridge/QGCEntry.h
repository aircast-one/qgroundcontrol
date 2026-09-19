#pragma once

#ifdef __cplusplus
extern "C" {
#endif

int qgc_start(int argc, char *argv[]);
int qgc_run(void);
void qgc_shutdown(void);

void qgc_set_host_provides_ui(int provides);
void qgc_request_quit(void);

#ifdef __cplusplus
}
#endif

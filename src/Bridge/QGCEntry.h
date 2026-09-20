#pragma once

#ifdef __cplusplus
extern "C" {
#endif

int qgc_start(int argc, char *argv[]);
int qgc_run(void);
void qgc_shutdown(void);

int qgc_runs_event_loop(void);

int qgc_core_headless(void);

void qgc_set_host_provides_ui(int provides);
void qgc_request_quit(void);

void qgc_handle_deep_link(const char *url);

#ifdef __cplusplus
}
#endif

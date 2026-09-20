#pragma once

#ifdef __cplusplus
extern "C" {
#endif

int qgc_start(int argc, char *argv[]);
int qgc_run(void);
void qgc_shutdown(void);

/// True when qgc_run() will enter the Qt event loop rather than finishing on its own
/// (--simple-boot-test, --list-tests, unit tests). Valid only after qgc_start().
int qgc_runs_event_loop(void);

/// True when this library was built without Qt's GUI stack. A headless core creates a
/// QCoreApplication, so the host owns the main thread and Qt's loop runs on its own.
int qgc_core_headless(void);

void qgc_set_host_provides_ui(int provides);
void qgc_request_quit(void);

/// Hands an aircast-qgc:// URL to the core. A host that owns the application object delivers
/// the OS open-url event itself; Qt's own file-open event never reaches a QCoreApplication.
void qgc_handle_deep_link(const char *url);

#ifdef __cplusplus
}
#endif

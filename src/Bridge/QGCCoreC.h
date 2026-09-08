#pragma once

#ifdef __cplusplus
extern "C" {
#endif

typedef void (*QGCCoreEventFn)(const char *path, const char *json);

char *qgc_core_get(const char *path);
char *qgc_core_get_fields(const char *path, const char *fields_csv);
char *qgc_core_set(const char *path, const char *value_json);
char *qgc_core_invoke(const char *path, const char *args_json);
void qgc_core_watch(const char *paths_csv);
void qgc_core_watch_client(const char *client, const char *paths_csv);
void qgc_core_set_event_handler(QGCCoreEventFn handler);
void qgc_core_free(char *text);

char *qgc_qt_get(const char *path);
char *qgc_qt_get_fields(const char *path, const char *fields_csv);
char *qgc_qt_set(const char *path, const char *value_json);
char *qgc_qt_invoke(const char *path, const char *args_json);
void qgc_qt_watch(const char *paths_csv);
void qgc_qt_set_event_handler(QGCCoreEventFn handler);
void qgc_qt_free(char *text);

#ifdef __cplusplus
}
#endif

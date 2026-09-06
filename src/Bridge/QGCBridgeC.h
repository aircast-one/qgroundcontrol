#pragma once

#ifdef __cplusplus
extern "C" {
#endif

typedef void (*QGCBridgeEventFn)(const char *path, const char *json);

char *qgc_bridge_get(const char *path);
char *qgc_bridge_set(const char *path, const char *value_json);
char *qgc_bridge_invoke(const char *path, const char *args_json);
void qgc_bridge_watch(const char *paths_csv);
void qgc_bridge_set_event_handler(QGCBridgeEventFn handler);
void qgc_bridge_free(char *text);

#ifdef __cplusplus
}
#endif

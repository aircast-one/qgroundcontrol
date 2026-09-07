#pragma once

#ifdef __cplusplus
extern "C" {
#endif

typedef struct QGCNativeDebugHooks {
    char *(*windows)(void);
    char *(*click)(const char *window, double x, double y);
    char *(*type)(const char *window, const char *text);
    char *(*probe)(const char *id, const char *action, const char *args_json);
    char *(*menu)(void);
    char *(*menu_invoke)(const char *path);
    char *(*bridge_stats)(void);
} QGCNativeDebugHooks;

void qgc_native_debug_install(const QGCNativeDebugHooks *hooks);
const QGCNativeDebugHooks *qgc_native_debug_hooks(void);

#ifdef __cplusplus
}
#endif

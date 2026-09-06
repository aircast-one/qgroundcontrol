#pragma once

#ifdef __cplusplus
extern "C" {
#endif

char *qgc_native_windows(void);
char *qgc_native_click(const char *window, double x, double y);
char *qgc_native_menu(void);
char *qgc_native_menu_invoke(const char *path);
char *qgc_native_bridge_stats(void);

#ifdef __cplusplus
}
#endif

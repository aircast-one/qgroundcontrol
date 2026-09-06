#pragma once

#ifdef __cplusplus
extern "C" {
#endif

int qgc_start(int argc, char *argv[]);
int qgc_run(void);
void qgc_shutdown(void);

void qgc_embed_main_window(void *native_view);
void qgc_resize_main_window(int width, int height);

#ifdef __cplusplus
}
#endif

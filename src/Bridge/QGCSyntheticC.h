#pragma once

#ifdef __cplusplus
extern "C" {
#endif

int qgc_synthetic_attach(void *metalLayer, int widthPx, int heightPx);

void qgc_synthetic_release(void *metalLayer);

#ifdef __cplusplus
}
#endif

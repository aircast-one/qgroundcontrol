#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef void (*QGCTileHandler)(const unsigned char *bytes, int length, void *context);

void qgc_map_tile_fetch(const char *mapType, int x, int y, int zoom,
                        QGCTileHandler handler, void *context);

uint64_t qgc_map_tile_fetch_cancellable(const char *mapType, int x, int y, int zoom,
                                        QGCTileHandler handler, void *context);

void qgc_map_tile_cancel(uint64_t ticket);

char *qgc_map_current_type(void);

#ifdef __cplusplus
}
#endif

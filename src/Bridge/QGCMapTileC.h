#pragma once

#ifdef __cplusplus
extern "C" {
#endif

// Serves tiles out of QGC's existing SQLite cache so a native map can draw the same
// imagery offline that the QML map already downloaded. Asynchronous because
// MKTileOverlay's loadTile is, and because the cache is read on its own thread.
//
// handler is called exactly once per request, with bytes == 0 when the tile is not
// cached, and always on the Qt thread.
typedef void (*QGCTileHandler)(const unsigned char *bytes, int length, void *context);

void qgc_map_tile_fetch(const char *mapType, int x, int y, int zoom,
                        QGCTileHandler handler, void *context);

// The imagery the operator has chosen, so the native map draws what QGC would.
char *qgc_map_current_type(void);

#ifdef __cplusplus
}
#endif

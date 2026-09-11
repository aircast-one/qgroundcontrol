#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

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

char *qgc_core_tile_open(const char *path);
void qgc_core_tile_close(void);
long long qgc_core_tile_size(const char *hash);
long long qgc_core_tile_copy(const char *hash, unsigned char *into, long long capacity);
char *qgc_core_tile_hash(int provider, int x, int y, int z);
int qgc_core_tile_provider(const char *name);


typedef void (*QGCCoreLinkWriterFn)(uint32_t id, const uint8_t *bytes, size_t len, void *user);

char *qgc_core_link_open(const char *config_json);
bool qgc_core_link_close(uint32_t id, const char *reason);
bool qgc_core_link_write(uint32_t id, const uint8_t *bytes, size_t len);
char *qgc_core_host_link_open(const char *kind, const char *name);
void qgc_core_host_link_bytes(uint32_t id, const uint8_t *bytes, size_t len);
bool qgc_core_host_link_closed(uint32_t id, const char *reason);
void qgc_core_set_link_writer(QGCCoreLinkWriterFn writer, void *user);
void qgc_core_link_announce_on_state(void);
char *qgc_core_guided(const char *action_json);
char *qgc_core_parameter(const char *request_json);
char *qgc_core_mission(const char *request_json);
char *qgc_core_remote_id(const char *request_json);
char *qgc_core_log(const char *request_json);
char *qgc_core_calibrate(const char *request_json);

typedef void (*QGCCoreLinkBytesSinkFn)(uint32_t id, const uint8_t *bytes, size_t len, void *user);
typedef void (*QGCCoreLinkStateSinkFn)(uint32_t id, bool open, const char *reason, void *user);
void qgc_core_set_link_bytes_sink(QGCCoreLinkBytesSinkFn sink, void *user);
void qgc_core_set_link_state_sink(QGCCoreLinkStateSinkFn sink, void *user);

char *qgc_qt_get(const char *path);
char *qgc_qt_get_fields(const char *path, const char *fields_csv);
char *qgc_qt_set(const char *path, const char *value_json);
char *qgc_qt_invoke(const char *path, const char *args_json);
void qgc_qt_watch(const char *paths_csv);
char *qgc_qt_watch_status(void);
void qgc_qt_set_event_handler(QGCCoreEventFn handler);
void qgc_qt_free(char *text);

#ifdef __cplusplus
}
#endif

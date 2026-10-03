#pragma once

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

bool qgc_video_available(void);
bool qgc_video_start(const char *pipelineDescription);
void qgc_video_stop(void);
bool qgc_video_running(void);
int qgc_video_width(void);
int qgc_video_height(void);
int64_t qgc_video_frames(void);
int64_t qgc_video_source_buffers(void);
const char *qgc_video_last_error(void);
bool qgc_video_copy_frame(void *destination, int capacity, int *width, int *height, int *stride);
bool qgc_video_attach_appsink(void *appsink);
void qgc_video_detach_appsink(void);
bool qgc_video_set_window(void *native_window);
bool qgc_video_attach_overlay(void *sink);
bool qgc_video_start_recording(const char *file, int format);
void qgc_video_stop_recording(void);
bool qgc_video_recording(void);

typedef void (*qgc_video_frame_callback)(const uint8_t *pixels, int width, int height, int stride);
void qgc_video_set_frame_callback(qgc_video_frame_callback callback);

typedef void (*qgc_video_pipeline_callback)(void *pipeline);
void qgc_video_set_pipeline_callback(qgc_video_pipeline_callback callback);

#ifdef __cplusplus
}
#endif

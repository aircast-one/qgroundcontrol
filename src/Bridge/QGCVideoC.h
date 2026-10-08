#pragma once

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define QGC_VIDEO_MAIN 0
#define QGC_VIDEO_PIP 1
#define QGC_VIDEO_CHANNELS 2

int qgc_video_abi_version(void);
bool qgc_video_available(void);
bool qgc_video_start(int channel, const char *pipelineDescription);
void qgc_video_stop(int channel);
bool qgc_video_running(int channel);
int qgc_video_width(int channel);
int qgc_video_height(int channel);
int64_t qgc_video_frames(int channel);
int64_t qgc_video_source_buffers(int channel);
const char *qgc_video_last_error(int channel);
const char *qgc_video_stream_error(int channel);
bool qgc_video_copy_frame(int channel, void *destination, int capacity, int *width, int *height, int *stride);
bool qgc_video_attach_appsink(int channel, void *appsink);
void qgc_video_detach_appsink(int channel);
bool qgc_video_set_window(int channel, void *native_window);
bool qgc_video_attach_overlay(int channel, void *sink);
bool qgc_video_start_recording(int channel, const char *file, int format);
void qgc_video_stop_recording(int channel);
bool qgc_video_recording(int channel);

typedef void (*qgc_video_pipeline_callback)(int channel, void *pipeline);
void qgc_video_set_pipeline_callback(qgc_video_pipeline_callback callback);

#ifdef __cplusplus
}
#endif

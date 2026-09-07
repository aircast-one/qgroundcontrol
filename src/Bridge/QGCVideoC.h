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
const char *qgc_video_last_error(void);
bool qgc_video_copy_frame(void *destination, int capacity, int *width, int *height, int *stride);

#ifdef __cplusplus
}
#endif

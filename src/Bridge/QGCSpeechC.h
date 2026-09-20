#pragma once

#ifdef __cplusplus
extern "C" {
#endif

typedef void (*QGCSpeechHandler)(const char *text, double volume);

void qgc_set_speech_handler(QGCSpeechHandler handler);

void qgc_speak(const char *text, double volume);

int qgc_speech_available(void);

#ifdef __cplusplus
}
#endif

#pragma once

#ifdef __cplusplus
extern "C" {
#endif

typedef void (*QGCSpeechHandler)(const char *text, double volume);

/// The headless core links no speech engine - QtTextToSpeech pulls QtGui and QtMultimedia in
/// with it - so the host speaks. Pass nullptr to stop speaking.
void qgc_set_speech_handler(QGCSpeechHandler handler);

/// Hands @p text to the registered handler. No-op when none is registered.
void qgc_speak(const char *text, double volume);

/// True when a host has registered a handler.
int qgc_speech_available(void);

#ifdef __cplusplus
}
#endif

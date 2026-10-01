#pragma once

#include "QGCCoreC.h"

#ifdef __cplusplus
extern "C" {
#endif

const QGCPacketRadioNative *qgc_wfb_native(void);
bool qgc_wfb_start_fd(int fd, uint8_t channel, int32_t channel_width, const char *key_path, char **error);
const char *qgc_wfb_adapter_name(uint16_t vendor_id, uint16_t product_id);

#ifdef __cplusplus
}
#endif

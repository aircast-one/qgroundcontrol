#pragma once

#ifdef __cplusplus
extern "C" {
#endif

// Creating a link is a transaction: createConfiguration hands back an object that is
// not in linkConfigurations yet, so the path-addressed bridge has nothing to name it by
// until endCreateConfiguration adopts it. That is why creation is typed.
//
// connect/disconnect/remove are NOT here any more. They looked impossible generically
// because "@path" object arguments appeared not to convert, but the arguments were
// simply never arriving: the debug API read its query values with QUrl::PrettyDecoded,
// which leaves brackets percent-encoded, so every invoke ran with an empty argument
// list. Those three go through the generic bridge.
int qgc_links_create(int type, const char *name, const char *host, int port);

#ifdef __cplusplus
}
#endif

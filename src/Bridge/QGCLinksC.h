#pragma once

#ifdef __cplusplus
extern "C" {
#endif

// The generic bridge resolves properties and no-argument invokables, but cannot pass a
// LinkConfiguration to LinkManager: an "@path" argument arrives as QVariant(QObject *)
// and will not convert to "const LinkConfiguration *". Link lifecycle is a typed API
// rather than a property, so it gets typed entry points. Property edits (name, host,
// port, autoConnect) still go through the generic bridge.
int qgc_links_connect(int index);
int qgc_links_disconnect(int index);
int qgc_links_remove(int index);
int qgc_links_create(int type, const char *name, const char *host, int port);

#ifdef __cplusplus
}
#endif

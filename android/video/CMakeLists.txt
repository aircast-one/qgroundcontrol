cmake_minimum_required(VERSION 3.25)
project(qgc_video LANGUAGES C CXX)

set(QGC_ROOT "${CMAKE_CURRENT_LIST_DIR}/../..")
list(APPEND CMAKE_MODULE_PATH "${QGC_ROOT}/cmake" "${QGC_ROOT}/cmake/modules" "${QGC_ROOT}/cmake/find-modules")
set(CPM_SOURCE_CACHE "${QGC_ROOT}/.cache/CPM" CACHE PATH "CPM source cache directory")
include(CPM)

file(READ "${QGC_ROOT}/.github/build-config.json" QGC_BUILD_CONFIG_CONTENT)
string(JSON GStreamer_FIND_VERSION GET "${QGC_BUILD_CONFIG_CONTENT}" gstreamer version android)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY BOTH)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE BOTH)
set(CMAKE_FIND_ROOT_PATH_MODE_PACKAGE BOTH)
option(QGC_MOCK_VIDEO_SERVER "Serve the MockLink test video stream" OFF)
set(QGCGStreamer_FIND_COMPONENTS Core Base Video App Rtsp)
if(QGC_MOCK_VIDEO_SERVER)
    list(APPEND QGCGStreamer_FIND_COMPONENTS RtspServer)
endif()
include(GStreamer/Orchestrator)

set(_qgc_gst_template "${QGC_ROOT}/src/VideoManager/VideoReceiver/GStreamer/gst_static_plugins.c.in")
configure_file("${_qgc_gst_template}" "${CMAKE_CURRENT_BINARY_DIR}/gst_static_plugins.c" @ONLY)

add_library(qgc_video SHARED
    "${QGC_ROOT}/src/Bridge/QGCVideoC.cc"
    "${CMAKE_CURRENT_BINARY_DIR}/gst_static_plugins.c"
    QGCVideoAndroid.cc
)
if(QGC_MOCK_VIDEO_SERVER)
    target_sources(qgc_video PRIVATE QGCMockVideoServer.cc)
endif()
target_include_directories(qgc_video PRIVATE "${QGC_ROOT}/src/Bridge")
target_compile_definitions(qgc_video PRIVATE QGC_GST_STREAMING QGC_VIDEO_FORMAT="RGBA")
target_link_libraries(qgc_video PRIVATE GStreamer::GStreamer GStreamer::mobile android log)
add_dependencies(qgc_video copyjavasource_${CMAKE_ANDROID_ARCH_ABI})

add_subdirectory("${QGC_ROOT}/libs/wfb" wfb)

add_library(qgc_wfb SHARED "${QGC_ROOT}/src/Bridge/QGCWfbC.cc")
target_include_directories(qgc_wfb PRIVATE "${QGC_ROOT}/src/Bridge")
target_link_libraries(qgc_wfb PRIVATE wfblink log)

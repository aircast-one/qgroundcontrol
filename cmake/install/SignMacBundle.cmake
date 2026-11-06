message(STATUS "QGC: Signing Bundle using signing identity")

include(ProcessorCount)
ProcessorCount(_nproc)
if(_nproc EQUAL 0)
    set(_nproc 4)
endif()

function(_qgc_apple_retry what)
    foreach(_attempt RANGE 1 6)
        execute_process(COMMAND ${ARGN} RESULT_VARIABLE _result)
        if(_result EQUAL 0)
            return()
        endif()
        message(STATUS "QGC: ${what} failed (attempt ${_attempt} of 6), retrying in 20 s")
        execute_process(COMMAND sleep 20)
    endforeach()
    message(FATAL_ERROR "QGC: ${what} failed after 6 attempts")
endfunction()

function(_parallel_codesign IDENTITY)
    string(RANDOM LENGTH 8 _cs_rand)
    set(_cs_list "${CMAKE_BINARY_DIR}/_codesign_${_cs_rand}.bin")
    execute_process(
        COMMAND find ${ARGN} -print0
        OUTPUT_FILE "${_cs_list}"
        RESULT_VARIABLE _find_rc
    )
    if(NOT _find_rc EQUAL 0)
        message(FATAL_ERROR "QGC: find failed during codesign (exit ${_find_rc})")
    endif()
    file(SIZE "${_cs_list}" _cs_size)
    if(_cs_size GREATER 0)
        execute_process(
            COMMAND xargs -0 -P ${_nproc} -n 1 sh -c "for attempt in 1 2 3 4 5 6; do codesign --timestamp --options=runtime --force -s \"$0\" \"$1\" && exit 0; echo \"QGC: signing $1 failed (attempt $attempt of 6), retrying in 20 s\" >&2; sleep 20; done; exit 1" "${IDENTITY}"
            INPUT_FILE "${_cs_list}"
            COMMAND_ERROR_IS_FATAL ANY
        )
    endif()
    file(REMOVE "${_cs_list}")
endfunction()

if(NOT DEFINED ENV{QGC_MACOS_SIGNING_IDENTITY} OR "$ENV{QGC_MACOS_SIGNING_IDENTITY}" STREQUAL "")
    message(FATAL_ERROR "QGC: QGC_MACOS_SIGNING_IDENTITY environment variable must be set to sign MacOS bundle")
endif()
if(NOT DEFINED ENV{QGC_MACOS_NOTARIZATION_USERNAME} OR "$ENV{QGC_MACOS_NOTARIZATION_USERNAME}" STREQUAL "")
    message(FATAL_ERROR "QGC: QGC_MACOS_NOTARIZATION_USERNAME environment variable must be set to notarize MacOS bundle")
endif()
if(NOT DEFINED ENV{QGC_MACOS_NOTARIZATION_TEAM_ID} OR "$ENV{QGC_MACOS_NOTARIZATION_TEAM_ID}" STREQUAL "")
    message(FATAL_ERROR "QGC: QGC_MACOS_NOTARIZATION_TEAM_ID environment variable must be set to notarize MacOS bundle")
endif()
if(NOT DEFINED ENV{QGC_MACOS_NOTARIZATION_PASSWORD} OR "$ENV{QGC_MACOS_NOTARIZATION_PASSWORD}" STREQUAL "")
    message(FATAL_ERROR "QGC: QGC_MACOS_NOTARIZATION_PASSWORD environment variable must be set to notarize MacOS bundle")
endif()

_parallel_codesign("$ENV{QGC_MACOS_SIGNING_IDENTITY}"
    "${QGC_STAGING_BUNDLE_PATH}/Contents" -path "*/Frameworks/*.framework" -prune -o -type f "(" -name "*.dylib" -o -name "*.so" ")"
)

if(EXISTS "${QGC_STAGING_BUNDLE_PATH}/Contents/Frameworks/GStreamer.framework")
    _parallel_codesign("$ENV{QGC_MACOS_SIGNING_IDENTITY}"
        "${QGC_STAGING_BUNDLE_PATH}/Contents/Frameworks/GStreamer.framework/Versions/1.0/libexec/gstreamer-1.0" -type f
    )
    _qgc_apple_retry("Signing GStreamer"
        codesign --timestamp --options=runtime --force -s "$ENV{QGC_MACOS_SIGNING_IDENTITY}" "${QGC_STAGING_BUNDLE_PATH}/Contents/Frameworks/GStreamer.framework/Versions/1.0/lib/GStreamer"
    )
    _qgc_apple_retry("Signing GStreamer"
        codesign --timestamp --options=runtime --force -s "$ENV{QGC_MACOS_SIGNING_IDENTITY}" "${QGC_STAGING_BUNDLE_PATH}/Contents/Frameworks/GStreamer.framework/Versions/1.0/GStreamer"
    )
endif()

if(EXISTS "${QGC_STAGING_BUNDLE_PATH}/Contents/libexec/gstreamer-1.0")
    _parallel_codesign("$ENV{QGC_MACOS_SIGNING_IDENTITY}"
        "${QGC_STAGING_BUNDLE_PATH}/Contents/libexec/gstreamer-1.0" -type f -perm +111
    )
endif()

file(GLOB FRAMEWORK_DIRS "${QGC_STAGING_BUNDLE_PATH}/Contents/Frameworks/*.framework")
foreach(FRAMEWORK_DIR ${FRAMEWORK_DIRS})
    _parallel_codesign("$ENV{QGC_MACOS_SIGNING_IDENTITY}"
        "${FRAMEWORK_DIR}" -type f "(" -name "*.dylib" -o -name "*.so" -o -perm +111 ")"
    )
    _qgc_apple_retry("Signing ${FRAMEWORK_DIR}"
        codesign --timestamp --options=runtime --force -s "$ENV{QGC_MACOS_SIGNING_IDENTITY}" "${FRAMEWORK_DIR}"
    )
endforeach()

if(EXISTS "${QGC_STAGING_BUNDLE_PATH}/Contents/Frameworks/GStreamer.framework")
    execute_process(
        COMMAND codesign --verify --deep --strict --verbose=2 "${QGC_STAGING_BUNDLE_PATH}/Contents/Frameworks/GStreamer.framework"
        COMMAND_ERROR_IS_FATAL ANY
    )
endif()

_qgc_apple_retry("Signing ${QGC_STAGING_BUNDLE_PATH}"
    codesign --timestamp --options=runtime --force -s "$ENV{QGC_MACOS_SIGNING_IDENTITY}" "${QGC_STAGING_BUNDLE_PATH}"
)

execute_process(
    COMMAND codesign --verify --deep --strict --verbose=2 "${QGC_STAGING_BUNDLE_PATH}"
    COMMAND_ERROR_IS_FATAL ANY
)

set(_notarize_zip "${CMAKE_BINARY_DIR}/qgc_notarization_upload.zip")
message(STATUS "QGC: Archiving Bundle for Notarization upload")

file(REMOVE "${_notarize_zip}")
execute_process(
    COMMAND ditto -c -k --keepParent "${QGC_STAGING_BUNDLE_PATH}" "${_notarize_zip}"
    COMMAND_ERROR_IS_FATAL ANY
)
message(STATUS "QGC: Notarizing app bundle. This may take a while...")
_qgc_apple_retry("Notarizing"
    xcrun notarytool submit "${_notarize_zip}" --apple-id "$ENV{QGC_MACOS_NOTARIZATION_USERNAME}" --team-id "$ENV{QGC_MACOS_NOTARIZATION_TEAM_ID}" --password "$ENV{QGC_MACOS_NOTARIZATION_PASSWORD}" --wait
)
message(STATUS "QGC: Stapling notarization ticket to app bundle")
_qgc_apple_retry("Stapling" xcrun stapler staple "${QGC_STAGING_BUNDLE_PATH}")

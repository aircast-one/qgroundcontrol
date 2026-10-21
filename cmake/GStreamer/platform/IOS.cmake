macro(_qgc_discover_ios_sdk)
    if(NOT CMAKE_HOST_APPLE)
        message(FATAL_ERROR "GStreamer for iOS can only be built on macOS")
    endif()

    if(GStreamer_FIND_VERSION VERSION_LESS "1.28.0")
        message(FATAL_ERROR
            "GStreamer for iOS requires version 1.28 or later. "
            "Got '${GStreamer_FIND_VERSION}' — bump gstreamer.version.ios in build-config.json.")
    endif()

    set(_gst_ios_bundle "")
    set(_gst_ios_bundle_kind "")

    if(NOT DEFINED GStreamer_ROOT_DIR)
        if(EXISTS "/Library/Developer/GStreamer/iPhone.sdk/GStreamer.xcframework")
            set(_gst_ios_bundle "/Library/Developer/GStreamer/iPhone.sdk/GStreamer.xcframework")
            set(_gst_ios_bundle_kind "xcframework")
        elseif(EXISTS "/Library/Developer/GStreamer/iPhone.sdk/GStreamer.framework")
            set(_gst_ios_bundle "/Library/Developer/GStreamer/iPhone.sdk/GStreamer.framework")
            set(_gst_ios_bundle_kind "framework")
        endif()
    endif()

    if(NOT _gst_ios_bundle)
        gstreamer_resolve_or_download_sdk(
            PLATFORM         ios
            CACHE_SUBDIR     "gstreamer-ios-${GStreamer_FIND_VERSION}"
            FILENAME_PRIMARY "gstreamer-ios.pkg"
            CACHE_DIR_OUT    _gst_ios_cache_dir
            ARCHIVE_OUT      _gst_ios_pkg
        )
        set(_gst_ios_expanded "${_gst_ios_cache_dir}/expanded")

        set(_gst_ios_anchor_globs
            "${_gst_ios_expanded}/*/GStreamer.xcframework/Info.plist"
            "${_gst_ios_expanded}/GStreamer.xcframework/Info.plist"
            "${_gst_ios_expanded}/*/GStreamer.framework/Versions/1.0/GStreamer"
            "${_gst_ios_expanded}/*/GStreamer.framework/GStreamer"
            "${_gst_ios_expanded}/*/GStreamer.framework/Versions/1.0/Headers/gst/gst.h"
            "${_gst_ios_expanded}/*/GStreamer.framework/Headers/gst/gst.h"
        )

        if(EXISTS "${_gst_ios_expanded}")
            set(_cached_anchor "")
            foreach(_glob IN LISTS _gst_ios_anchor_globs)
                file(GLOB_RECURSE _cached_anchor "${_glob}")
                if(_cached_anchor)
                    break()
                endif()
            endforeach()
            if(NOT _cached_anchor)
                message(STATUS "GStreamer: cached iOS expansion is incomplete; re-expanding")
                file(REMOVE_RECURSE "${_gst_ios_expanded}")
            endif()
        endif()

        if(NOT EXISTS "${_gst_ios_expanded}")
            _qgc_pkgutil_expand_and_validate("${_gst_ios_pkg}" "${_gst_ios_expanded}" "iOS")
        endif()

        file(GLOB_RECURSE _xcfw_info_plists LIST_DIRECTORIES false
            "${_gst_ios_expanded}/*/GStreamer.xcframework/Info.plist"
            "${_gst_ios_expanded}/GStreamer.xcframework/Info.plist"
        )
        if(_xcfw_info_plists)
            list(GET _xcfw_info_plists 0 _xcfw_info_first)
            cmake_path(GET _xcfw_info_first PARENT_PATH _gst_ios_bundle)
            set(_gst_ios_bundle_kind "xcframework")
        else()
            file(GLOB_RECURSE _fw_dirs LIST_DIRECTORIES true
                "${_gst_ios_expanded}/*/GStreamer.framework"
                "${_gst_ios_expanded}/GStreamer.framework")
            foreach(_d IN LISTS _fw_dirs)
                if(IS_DIRECTORY "${_d}" AND _d MATCHES "/GStreamer\\.framework$")
                    set(_gst_ios_bundle "${_d}")
                    set(_gst_ios_bundle_kind "framework")
                    break()
                endif()
            endforeach()
        endif()

        if(NOT _gst_ios_bundle)
            file(GLOB _top_entries LIST_DIRECTORIES true "${_gst_ios_expanded}/*")
            file(GLOB_RECURSE _all_xcfw LIST_DIRECTORIES true "${_gst_ios_expanded}/*.xcframework")
            file(GLOB_RECURSE _all_fw   LIST_DIRECTORIES true "${_gst_ios_expanded}/*.framework")
            string(REPLACE ";" "\n  " _top_entries_str "${_top_entries}")
            string(REPLACE ";" "\n  " _all_xcfw_str "${_all_xcfw}")
            string(REPLACE ";" "\n  " _all_fw_str "${_all_fw}")
            message(FATAL_ERROR
                "Could not locate GStreamer.xcframework or GStreamer.framework in expanded iOS SDK at"
                " '${_gst_ios_expanded}'. The .pkg layout may have changed.\n"
                "Top-level entries:\n  ${_top_entries_str}\n"
                "All *.xcframework directories:\n  ${_all_xcfw_str}\n"
                "All *.framework directories:\n  ${_all_fw_str}")
        endif()
        set(GStreamer_AUTO_DOWNLOADED TRUE)
    endif()

    set(GStreamer_USE_XCFRAMEWORK ON)
    set(GStreamer_USE_FRAMEWORK OFF)
    set(GStreamer_USE_STATIC_LIBS OFF)

    if(_gst_ios_bundle_kind STREQUAL "xcframework")
        string(TOLOWER "${CMAKE_OSX_SYSROOT}" _gst_ios_sysroot_lc)
        if(_gst_ios_sysroot_lc MATCHES "simulator")
            file(GLOB _gst_ios_slice_dirs LIST_DIRECTORIES true "${_gst_ios_bundle}/ios-*-simulator")
        else()
            file(GLOB _gst_ios_slice_dirs LIST_DIRECTORIES true "${_gst_ios_bundle}/ios-*")
            list(FILTER _gst_ios_slice_dirs EXCLUDE REGEX "-(simulator|maccatalyst)$")
        endif()
        if(NOT _gst_ios_slice_dirs)
            message(FATAL_ERROR
                "GStreamer xcframework: no matching iOS slice found in ${_gst_ios_bundle}.\n"
                "Check ${_gst_ios_bundle}/Info.plist AvailableLibraries for the available slices.")
        endif()
        set(_gst_ios_slice_dir "")
        if(CMAKE_OSX_ARCHITECTURES)
            foreach(_slice IN LISTS _gst_ios_slice_dirs)
                foreach(_arch IN LISTS CMAKE_OSX_ARCHITECTURES)
                    if(_slice MATCHES "(^|[-_/])${_arch}([-_]|$)")
                        set(_gst_ios_slice_dir "${_slice}")
                        break()
                    endif()
                endforeach()
                if(_gst_ios_slice_dir)
                    break()
                endif()
            endforeach()
        endif()
        if(NOT _gst_ios_slice_dir)
            list(GET _gst_ios_slice_dirs 0 _gst_ios_slice_dir)
        endif()
        if(NOT EXISTS "${_gst_ios_slice_dir}")
            message(FATAL_ERROR
                "GStreamer xcframework slice '${_gst_ios_slice_dir}' is not a directory.")
        endif()
        set(GStreamer_ROOT_DIR        "${_gst_ios_slice_dir}")
        set(GSTREAMER_XCFRAMEWORK_LIB "${_gst_ios_slice_dir}/libGStreamer.a")
        set(_gst_ios_include          "${_gst_ios_slice_dir}/Headers")
        set(_gst_ios_layout_root      "${_gst_ios_slice_dir}")
        set(_gst_ios_xcfw_bundle      "${_gst_ios_bundle}")
    else()
        if(EXISTS "${_gst_ios_bundle}/Versions/1.0/GStreamer")
            set(_gst_ios_layout_root "${_gst_ios_bundle}/Versions/1.0")
        elseif(EXISTS "${_gst_ios_bundle}/GStreamer")
            set(_gst_ios_layout_root "${_gst_ios_bundle}")
        else()
            message(FATAL_ERROR
                "GStreamer.framework at ${_gst_ios_bundle} has no merged binary at "
                "Versions/1.0/GStreamer or GStreamer. The SDK layout may have changed.")
        endif()
        set(GStreamer_ROOT_DIR        "${_gst_ios_layout_root}")
        set(GSTREAMER_XCFRAMEWORK_LIB "${_gst_ios_layout_root}/GStreamer")
        if(EXISTS "${_gst_ios_layout_root}/Headers")
            set(_gst_ios_include "${_gst_ios_layout_root}/Headers")
        elseif(EXISTS "${_gst_ios_bundle}/Headers")
            set(_gst_ios_include "${_gst_ios_bundle}/Headers")
        else()
            message(FATAL_ERROR
                "GStreamer.framework at ${_gst_ios_bundle} has no Headers/ directory.")
        endif()
        set(_gst_ios_xcfw_bundle     "${_gst_ios_bundle}")
    endif()

    if(NOT EXISTS "${GSTREAMER_XCFRAMEWORK_LIB}")
        message(FATAL_ERROR
            "GStreamer iOS merged binary not found at ${GSTREAMER_XCFRAMEWORK_LIB}")
    endif()

    set(GStreamer_IOS_INFO_PLIST "")
    if(_gst_ios_bundle_kind STREQUAL "xcframework")
        foreach(_pl
            "${_gst_ios_slice_dir}/GStreamer.framework/Info.plist"
            "${_gst_ios_slice_dir}/GStreamer.framework/Resources/Info.plist")
            if(EXISTS "${_pl}")
                set(GStreamer_IOS_INFO_PLIST "${_pl}")
                break()
            endif()
        endforeach()
    else()
        foreach(_pl
            "${_gst_ios_layout_root}/Resources/Info.plist"
            "${_gst_ios_bundle}/Resources/Info.plist"
            "${_gst_ios_bundle}/Info.plist")
            if(EXISTS "${_pl}")
                set(GStreamer_IOS_INFO_PLIST "${_pl}")
                break()
            endif()
        endforeach()
    endif()

    gstreamer_create_layout_target(
        SDK_ROOT           "${_gst_ios_layout_root}"
        TYPE               XCFRAMEWORK
        INCLUDE_PATH       "${_gst_ios_include}"
        XCFRAMEWORK_BUNDLE "${_gst_ios_xcfw_bundle}"
    )

    _qgc_download_ios_ca_bundle()
endmacro()

function(_qgc_download_ios_ca_bundle)
    if(CPM_SOURCE_CACHE)
        set(_ca_dir "${CPM_SOURCE_CACHE}/gstreamer-ios-ca")
    else()
        set(_ca_dir "${CMAKE_BINARY_DIR}/_deps/gstreamer-ios-ca")
    endif()

    set(_ca_hash_arg "")
    set(_ca_url "https://curl.se/ca/cacert.pem")
    if(QGC_BUILD_CONFIG_CONTENT)
        string(JSON _ca_sha256 ERROR_VARIABLE _ca_err
            GET "${QGC_BUILD_CONFIG_CONTENT}" "gstreamer" "ca_bundle_sha256")
        if(_ca_sha256 AND NOT _ca_err)
            set(_ca_hash_arg EXPECTED_HASH "SHA256=${_ca_sha256}")
        endif()
        string(JSON _ca_date ERROR_VARIABLE _ca_date_err
            GET "${QGC_BUILD_CONFIG_CONTENT}" "gstreamer" "ca_bundle_date")
        if(_ca_date AND NOT _ca_date_err)
            set(_ca_url "https://curl.se/ca/cacert-${_ca_date}.pem")
        endif()
    endif()
    if(NOT _ca_hash_arg)
        if(NOT QGC_GST_ALLOW_UNVERIFIED_CA)
            message(FATAL_ERROR
                "GStreamer: gstreamer.ca_bundle_sha256 is missing from build-config.json, so the "
                "iOS CA bundle (the app's TLS trust root) cannot be verified. Pin it from "
                "https://curl.se/ca/cacert.pem.sha256 and commit it to build-config.json. "
                "To fetch the bundle UNVERIFIED anyway (NOT for CI/release builds), set "
                "-DQGC_GST_ALLOW_UNVERIFIED_CA=ON.")
        endif()
        message(WARNING "GStreamer: gstreamer.ca_bundle_sha256 missing from build-config.json and "
            "QGC_GST_ALLOW_UNVERIFIED_CA is set; iOS CA bundle will be fetched UNVERIFIED. "
            "Pin it from https://curl.se/ca/cacert.pem.sha256")
    endif()

    qgc_resilient_download(
        FILENAME        ca-certificates.crt
        DESTINATION_DIR "${_ca_dir}"
        URLS            "${_ca_url}"
        RESULT_VAR      _ca_path
        LOG_TAG         "iOS CA bundle"
        FAILURE_HINT    "Network is required at first iOS configure to fetch the Mozilla CA bundle from curl.se."
        ${_ca_hash_arg}
    )

    set(GStreamer_IOS_CA_BUNDLE "${_ca_path}" CACHE FILEPATH
        "Mozilla CA bundle bundled into iOS app resources at ssl/certs/ca-certificates.crt" FORCE)
endfunction()

function(_qgc_create_xcframework_targets)
    set(_xcfw_bundle_version "")
    if(GStreamer_IOS_INFO_PLIST AND EXISTS "${GStreamer_IOS_INFO_PLIST}")
        find_program(_xcfw_plistbuddy NAMES PlistBuddy
            PATHS /usr/libexec NO_DEFAULT_PATH NO_CACHE)
        if(_xcfw_plistbuddy)
            execute_process(
                COMMAND "${_xcfw_plistbuddy}" -c "Print :CFBundleShortVersionString"
                        "${GStreamer_IOS_INFO_PLIST}"
                OUTPUT_VARIABLE _xcfw_bundle_version
                OUTPUT_STRIP_TRAILING_WHITESPACE
                ERROR_QUIET
                RESULT_VARIABLE _xcfw_plist_rc)
            if(NOT _xcfw_plist_rc EQUAL 0)
                set(_xcfw_bundle_version "")
            endif()
        endif()
        if(NOT _xcfw_bundle_version)
            file(READ "${GStreamer_IOS_INFO_PLIST}" _xcfw_plist_xml)
            string(REGEX MATCH
                "<key>CFBundleShortVersionString</key>[ \t\r\n]*<string>([^<]+)</string>"
                _xcfw_plist_match "${_xcfw_plist_xml}")
            if(_xcfw_plist_match)
                set(_xcfw_bundle_version "${CMAKE_MATCH_1}")
            endif()
        endif()
    endif()

    if(_xcfw_bundle_version)
        if(_xcfw_bundle_version VERSION_LESS "1.28.0")
            message(FATAL_ERROR
                "GStreamer iOS bundle reports version '${_xcfw_bundle_version}' "
                "(from ${GStreamer_IOS_INFO_PLIST}), which is older than the required "
                "1.28 floor. A downgraded or stale GStreamer.framework was found — "
                "remove it / clear the SDK cache and re-fetch the pinned version.")
        endif()
        if(GStreamer_FIND_VERSION AND _xcfw_bundle_version VERSION_LESS GStreamer_FIND_VERSION)
            message(WARNING
                "GStreamer iOS bundle version '${_xcfw_bundle_version}' is older than the "
                "requested '${GStreamer_FIND_VERSION}'. Using the bundle's actual version.")
        endif()
        set(GStreamer_VERSION "${_xcfw_bundle_version}" PARENT_SCOPE)
        set(GStreamer_VERSION "${_xcfw_bundle_version}")
    elseif(NOT GStreamer_VERSION)
        message(WARNING
            "GStreamer iOS: could not read CFBundleShortVersionString from the bundle "
            "Info.plist (looked at '${GStreamer_IOS_INFO_PLIST}'). Falling back to the "
            "REQUESTED version '${GStreamer_FIND_VERSION}' — the bundle's actual version "
            "is UNVERIFIED, so a downgraded SDK cannot be detected.")
        set(GStreamer_VERSION "${GStreamer_FIND_VERSION}" PARENT_SCOPE)
        set(GStreamer_VERSION "${GStreamer_FIND_VERSION}")
    endif()

    if(NOT TARGET GStreamer::GStreamer)
        add_library(GStreamer_static STATIC IMPORTED GLOBAL)
        set_target_properties(GStreamer_static PROPERTIES
            IMPORTED_LOCATION "${GSTREAMER_XCFRAMEWORK_LIB}"
        )
        add_library(GStreamer::GStreamer INTERFACE IMPORTED GLOBAL)
        target_link_libraries(GStreamer::GStreamer INTERFACE
            GStreamer_static
        )
        target_include_directories(GStreamer::GStreamer INTERFACE "${GSTREAMER_INCLUDE_PATH}")
        foreach(_inc_sub IN ITEMS gstreamer-1.0 glib-2.0)
            if(IS_DIRECTORY "${GSTREAMER_INCLUDE_PATH}/${_inc_sub}")
                target_include_directories(GStreamer::GStreamer INTERFACE "${GSTREAMER_INCLUDE_PATH}/${_inc_sub}")
            endif()
        endforeach()
        set(_xcfw_required_frameworks
            Foundation AVFoundation AudioToolbox VideoToolbox CoreMedia CoreVideo
            CoreAudio CoreGraphics Security UIKit CoreFoundation CoreText
            IOSurface Metal QuartzCore
        )
        set(_xcfw_resolved_libs)
        foreach(_fw IN LISTS _xcfw_required_frameworks)
            string(TOLOWER "_xcfw_${_fw}" _fw_var)
            find_library(${_fw_var} ${_fw} REQUIRED)
            list(APPEND _xcfw_resolved_libs "${${_fw_var}}")
        endforeach()
        find_library(GStreamer_xcfw_assetslibrary AssetsLibrary)
        set(_xcfw_assetslibrary "${GStreamer_xcfw_assetslibrary}")
        target_link_libraries(GStreamer::GStreamer INTERFACE
            ${_xcfw_resolved_libs}
            "-lresolv" "-liconv" "-lz" "-lbz2"
        )
        if(_xcfw_assetslibrary)
            target_link_libraries(GStreamer::GStreamer INTERFACE "${_xcfw_assetslibrary}")
        else()
            target_link_options(GStreamer::GStreamer INTERFACE "-Wl,-weak_framework,AssetsLibrary")
        endif()
        find_library(GStreamer_xcfw_opengles OpenGLES)
        set(_xcfw_opengles "${GStreamer_xcfw_opengles}")
        if(_xcfw_opengles)
            target_link_libraries(GStreamer::GStreamer INTERFACE "${_xcfw_opengles}")
        else()
            target_link_options(GStreamer::GStreamer INTERFACE "-Wl,-weak_framework,OpenGLES")
        endif()
        target_link_options(GStreamer::GStreamer INTERFACE "-Wl,-no_compact_unwind")
        target_compile_definitions(GStreamer::GStreamer INTERFACE
            QGC_GST_STATIC_BUILD
        )
    endif()

    foreach(_xcfw_comp IN LISTS GSTREAMER_APIS GSTREAMER_PLUGINS)
        if(NOT TARGET GStreamer::${_xcfw_comp})
            add_library(GStreamer::${_xcfw_comp} INTERFACE IMPORTED GLOBAL)
            target_link_libraries(GStreamer::${_xcfw_comp} INTERFACE GStreamer::GStreamer)
        endif()
        set(GStreamer_${_xcfw_comp}_FOUND TRUE PARENT_SCOPE)
    endforeach()

    if(NOT TARGET GStreamer::mobile)
        enable_language(OBJC OBJCXX)

        gstreamer_xcfw_skip(_xcfw_skip_plugins)

        file(SIZE "${GSTREAMER_XCFRAMEWORK_LIB}" _xcfw_lib_size)
        file(TIMESTAMP "${GSTREAMER_XCFRAMEWORK_LIB}" _xcfw_lib_mtime "%s")
        file(SHA256 "${GSTREAMER_XCFRAMEWORK_LIB}" _xcfw_lib_hash)
        set(_xcfw_cache_key "${GStreamer_VERSION}|${_xcfw_lib_size}|${_xcfw_lib_mtime}|${_xcfw_lib_hash}|${_xcfw_skip_plugins}")
        set(_xcfw_cache_file "${CMAKE_BINARY_DIR}/qgc_xcfw_plugins.cmake")
        set(_xcfw_descs "")
        if(EXISTS "${_xcfw_cache_file}")
            include("${_xcfw_cache_file}")
            if(NOT "${_xcfw_cached_key}" STREQUAL "${_xcfw_cache_key}")
                set(_xcfw_descs "")
            endif()
        endif()
        if(NOT _xcfw_descs)
            find_program(_xcfw_nm NAMES llvm-nm nm REQUIRED)

            execute_process(
                COMMAND "${_xcfw_nm}" -gjU "${GSTREAMER_XCFRAMEWORK_LIB}"
                OUTPUT_VARIABLE _xcfw_nm_out
                ERROR_VARIABLE _xcfw_nm_err
                RESULT_VARIABLE _xcfw_nm_rc
            )
            if(NOT _xcfw_nm_rc EQUAL 0)
                foreach(_xcfw_arch IN LISTS CMAKE_OSX_ARCHITECTURES)
                    execute_process(
                        COMMAND "${_xcfw_nm}" -arch "${_xcfw_arch}" -gjU "${GSTREAMER_XCFRAMEWORK_LIB}"
                        OUTPUT_VARIABLE _xcfw_nm_out
                        ERROR_VARIABLE _xcfw_nm_err
                        RESULT_VARIABLE _xcfw_nm_rc
                    )
                    if(_xcfw_nm_rc EQUAL 0)
                        break()
                    endif()
                endforeach()
            endif()

            string(REGEX MATCHALL "_gst_plugin_[A-Za-z0-9_]+_get_desc" _xcfw_descs "${_xcfw_nm_out}")
            list(REMOVE_DUPLICATES _xcfw_descs)
            if(NOT _xcfw_descs)
                message(FATAL_ERROR
                    "nm produced no plugin descriptors for ${GSTREAMER_XCFRAMEWORK_LIB} (rc=${_xcfw_nm_rc})\n"
                    "  tool:        ${_xcfw_nm}\n"
                    "  archs tried: ${CMAKE_OSX_ARCHITECTURES} (plus a no-arch attempt)\n"
                    "  stderr:      ${_xcfw_nm_err}\n"
                    "If this is a fat Mach-O, install llvm (brew install llvm) so "
                    "llvm-nm is on PATH, or verify the framework slice arch matches "
                    "CMAKE_OSX_ARCHITECTURES.")
            elseif(NOT _xcfw_nm_rc EQUAL 0)
                message(STATUS
                    "GStreamer xcframework: nm exited ${_xcfw_nm_rc} (unparseable objects, "
                    "e.g. Rust gstaws bitcode) but emitted usable symbols — continuing.")
            endif()
            file(WRITE "${_xcfw_cache_file}"
                "# Auto-generated cache of plugin descriptors enumerated from the iOS xcframework.\n"
                "# Regenerated when GStreamer_VERSION or libGStreamer.a (size+mtime) changes.\n"
                "set(_xcfw_cached_key \"${_xcfw_cache_key}\")\n"
                "set(_xcfw_descs \"${_xcfw_descs}\")\n"
            )
        endif()
        set(_xcfw_names "")
        foreach(_sym IN LISTS _xcfw_descs)
            string(REGEX REPLACE "^_gst_plugin_(.+)_get_desc$" "\\1" _name "${_sym}")
            if(_name IN_LIST _xcfw_skip_plugins)
                continue()
            endif()
            list(APPEND _xcfw_names "${_name}")
        endforeach()
        _gst_emit_static_plugin_registration(_xcfw_names _xcfw_decl _xcfw_reg)
        list(LENGTH _xcfw_names _xcfw_used)
        list(LENGTH _xcfw_descs _xcfw_n)
        message(STATUS "GStreamer xcframework: registering ${_xcfw_used}/${_xcfw_n} static plugins (skipped: ${_xcfw_skip_plugins})")
        set(PLUGINS_DECLARATION  "${_xcfw_decl}")
        set(PLUGINS_REGISTRATION "${_xcfw_reg}")
        set(G_IO_MODULES_DECLARE "GST_G_IO_MODULE_DECLARE(openssl);")
        set(G_IO_MODULES_LOAD    "    GST_G_IO_MODULE_LOAD(openssl);")

        set(_xcfw_static_shim "${CMAKE_BINARY_DIR}/${GStreamer_Mobile_MODULE_NAME}_static.c")
        set(_qgc_gst_src "${CMAKE_SOURCE_DIR}/src/VideoManager/VideoReceiver/GStreamer")
        configure_file(
            "${_qgc_gst_src}/gst_static_plugins.c.in"
            "${_xcfw_static_shim}"
            @ONLY
        )
        add_library(GStreamerMobileXcfw SHARED)
        target_sources(GStreamerMobileXcfw PRIVATE "${_xcfw_static_shim}")
        set_source_files_properties("${_xcfw_static_shim}" PROPERTIES GENERATED TRUE)
        target_link_libraries(GStreamerMobileXcfw PRIVATE GStreamer::GStreamer)
        set_target_properties(GStreamerMobileXcfw PROPERTIES
            LIBRARY_OUTPUT_NAME ${GStreamer_Mobile_MODULE_NAME}
            FRAMEWORK TRUE
            FRAMEWORK_VERSION A
            MACOSX_FRAMEWORK_IDENTIFIER org.gstreamer.GStreamerMobile
            LINKER_LANGUAGE CXX
        )
        add_library(GStreamer::mobile ALIAS GStreamerMobileXcfw)
        add_library(GStreamerMobile ALIAS GStreamerMobileXcfw)
        set(GStreamerMobile_FOUND TRUE PARENT_SCOPE)
        set(GStreamerMobile_mobile_FOUND TRUE PARENT_SCOPE)
    endif()

    set(GStreamer_FOUND TRUE PARENT_SCOPE)
endfunction()

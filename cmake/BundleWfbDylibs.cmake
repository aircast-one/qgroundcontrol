set(frameworks_dir "${APP_BUNDLE}/Contents/Frameworks")
file(MAKE_DIRECTORY "${frameworks_dir}")

execute_process(
    COMMAND otool -L "${APP_BINARY}"
    OUTPUT_VARIABLE otool_output
    OUTPUT_STRIP_TRAILING_WHITESPACE)

string(REPLACE "\n" ";" otool_lines "${otool_output}")

foreach(line IN LISTS otool_lines)
    string(REGEX MATCH "^[ \t]+([^ \t]+\\.dylib)" matched "${line}")
    if(NOT matched)
        continue()
    endif()
    set(dep "${CMAKE_MATCH_1}")

    if(NOT dep MATCHES "libusb|libsodium")
        continue()
    endif()
    if(dep MATCHES "^@")
        continue()
    endif()
    if(NOT EXISTS "${dep}")
        message(WARNING "wfb bundling: ${dep} not found, skipping")
        continue()
    endif()

    get_filename_component(dep_name "${dep}" NAME)
    file(COPY "${dep}" DESTINATION "${frameworks_dir}" FOLLOW_SYMLINK_CHAIN)
    execute_process(COMMAND chmod u+w "${frameworks_dir}/${dep_name}")
    execute_process(COMMAND install_name_tool -id "@rpath/${dep_name}" "${frameworks_dir}/${dep_name}")
    execute_process(COMMAND install_name_tool -change "${dep}" "@rpath/${dep_name}" "${APP_BINARY}")
    message(STATUS "wfb bundling: embedded ${dep_name}")
endforeach()

execute_process(COMMAND install_name_tool -add_rpath "@executable_path/../Frameworks" "${APP_BINARY}"
                ERROR_QUIET)

file(GLOB bundled_dylibs "${frameworks_dir}/libusb*.dylib" "${frameworks_dir}/libsodium*.dylib")
foreach(dylib IN LISTS bundled_dylibs)
    execute_process(COMMAND codesign --force --sign - "${dylib}"
                    RESULT_VARIABLE sign_result ERROR_VARIABLE sign_error)
    if(NOT sign_result EQUAL 0)
        message(FATAL_ERROR "wfb bundling: failed to sign ${dylib}: ${sign_error}")
    endif()
endforeach()

execute_process(COMMAND codesign --force --sign - "${APP_BUNDLE}"
                RESULT_VARIABLE sign_result ERROR_VARIABLE sign_error)
if(NOT sign_result EQUAL 0)
    message(FATAL_ERROR "wfb bundling: failed to re-sign app bundle: ${sign_error}")
endif()

execute_process(COMMAND codesign --verify "${APP_BUNDLE}"
                RESULT_VARIABLE verify_result ERROR_VARIABLE verify_error)
if(NOT verify_result EQUAL 0)
    message(FATAL_ERROR "wfb bundling: signature invalid after bundling: ${verify_error}")
endif()

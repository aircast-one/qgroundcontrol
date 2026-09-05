set(frameworks_dir "${APP_BUNDLE}/Contents/Frameworks")
file(MAKE_DIRECTORY "${frameworks_dir}")

# Same filesystem as the bundle, so the move into Frameworks is a rename and not a copy.
set(staging_dir "${APP_BUNDLE}/Contents/Frameworks/.wfb-staging")
file(REMOVE_RECURSE "${staging_dir}")
file(MAKE_DIRECTORY "${staging_dir}")

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

    # Patched and signed in a staging directory, then moved into place with a rename, which is
    # atomic on the same filesystem. Doing it in Frameworks left the dylib sitting there with a
    # signature that no longer matched its bytes for as long as install_name_tool and codesign
    # took to run - and this step runs on every build. A launch inside that window is killed by
    # the codesigning monitor before main: dyld maps the header, the kernel hashes the page, the
    # hash does not match, SIGKILL with CODESIGNING Code 2 Invalid Page.
    set(staged "${staging_dir}/${dep_name}")
    file(COPY "${dep}" DESTINATION "${staging_dir}" FOLLOW_SYMLINK_CHAIN)
    execute_process(COMMAND chmod u+w "${staged}")
    execute_process(COMMAND install_name_tool -id "@rpath/${dep_name}" "${staged}")

    execute_process(COMMAND codesign --force --sign - "${staged}"
                    RESULT_VARIABLE sign_result ERROR_VARIABLE sign_error)
    if(NOT sign_result EQUAL 0)
        message(FATAL_ERROR "wfb bundling: failed to sign ${dep_name}: ${sign_error}")
    endif()

    file(RENAME "${staged}" "${frameworks_dir}/${dep_name}")

    execute_process(COMMAND install_name_tool -change "${dep}" "@rpath/${dep_name}" "${APP_BINARY}")
    message(STATUS "wfb bundling: embedded ${dep_name}")
endforeach()

file(REMOVE_RECURSE "${staging_dir}")

execute_process(COMMAND install_name_tool -add_rpath "@executable_path/../Frameworks" "${APP_BINARY}"
                ERROR_QUIET)

execute_process(COMMAND codesign --force --sign - "${APP_BUNDLE}"
                RESULT_VARIABLE sign_result ERROR_VARIABLE sign_error)
if(NOT sign_result EQUAL 0)
    message(FATAL_ERROR "wfb bundling: failed to re-sign app bundle: ${sign_error}")
endif()

# --deep --strict, because plain --verify does not descend into nested code: a dylib in
# Frameworks whose signature did not match its bytes passed this check and then killed the app
# at launch.
execute_process(COMMAND codesign --verify --deep --strict "${APP_BUNDLE}"
                RESULT_VARIABLE verify_result ERROR_VARIABLE verify_error)
if(NOT verify_result EQUAL 0)
    message(FATAL_ERROR "wfb bundling: signature invalid after bundling: ${verify_error}")
endif()

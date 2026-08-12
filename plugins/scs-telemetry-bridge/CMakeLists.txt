cmake_minimum_required(VERSION 3.20)

project(OpenCarpanelScsTelemetryBridge VERSION 0.1.0 LANGUAGES CXX)

include(CTest)

function(opencarpanel_enable_warnings target)
  if(MSVC)
    target_compile_options(${target} PRIVATE /W4 /WX /permissive-)
  else()
    target_compile_options(${target} PRIVATE -Wall -Wextra -Wpedantic -Werror)
  endif()
endfunction()

add_library(opencarpanel_scs_bridge_protocol STATIC
  src/bridge_protocol.cpp
)
target_include_directories(opencarpanel_scs_bridge_protocol PUBLIC src)
target_compile_features(opencarpanel_scs_bridge_protocol PUBLIC cxx_std_17)
set_target_properties(opencarpanel_scs_bridge_protocol PROPERTIES
  CXX_EXTENSIONS OFF
  POSITION_INDEPENDENT_CODE ON
)

opencarpanel_enable_warnings(opencarpanel_scs_bridge_protocol)

add_library(opencarpanel_scs_telemetry SHARED
  src/plugin.cpp
)
target_link_libraries(opencarpanel_scs_telemetry PRIVATE
  opencarpanel_scs_bridge_protocol
)
target_include_directories(opencarpanel_scs_telemetry SYSTEM PRIVATE
  vendor/scs-sdk-1.14/include
)
target_compile_features(opencarpanel_scs_telemetry PRIVATE cxx_std_17)
target_compile_definitions(opencarpanel_scs_telemetry PRIVATE
  $<$<PLATFORM_ID:Windows>:WIN32_LEAN_AND_MEAN;NOMINMAX;_WIN32_WINNT=0x0601>
)
set_target_properties(opencarpanel_scs_telemetry PROPERTIES
  CXX_EXTENSIONS OFF
  CXX_VISIBILITY_PRESET hidden
  VISIBILITY_INLINES_HIDDEN YES
  PREFIX ""
  OUTPUT_NAME "opencarpanel-scs-telemetry"
)
opencarpanel_enable_warnings(opencarpanel_scs_telemetry)

if(WIN32)
  target_sources(opencarpanel_scs_telemetry PRIVATE src/plugin.def)
  target_link_libraries(opencarpanel_scs_telemetry PRIVATE ws2_32)
endif()

if(BUILD_TESTING)
  add_executable(opencarpanel_scs_bridge_protocol_test
    tests/bridge_protocol_test.cpp
  )
  target_link_libraries(opencarpanel_scs_bridge_protocol_test PRIVATE
    opencarpanel_scs_bridge_protocol
  )
  opencarpanel_enable_warnings(opencarpanel_scs_bridge_protocol_test)
  add_test(NAME opencarpanel_scs_bridge_protocol_test
    COMMAND opencarpanel_scs_bridge_protocol_test
  )
endif()

install(TARGETS opencarpanel_scs_telemetry
  RUNTIME DESTINATION .
  LIBRARY DESTINATION .
)
install(FILES README.md vendor/scs-sdk-1.14/sdk_license.txt DESTINATION .)

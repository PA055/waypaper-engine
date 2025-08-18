cmake_minimum_required(VERSION 3.16)

project(
    WaypaperEngine
    DESCRIPTION "A customizable, animated wallpaper inspired from wallpaper engine"
    VERSION 0.0.1
    LANGUAGES C CXX
)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Find required packages
find_package(PkgConfig REQUIRED)
pkg_check_modules(WAYLAND_CLIENT REQUIRED wayland-client)
pkg_check_modules(WAYLAND_PROTOCOLS REQUIRED wayland-protocols)

# Find wayland-scanner
find_program(WAYLAND_SCANNER wayland-scanner REQUIRED)

# Get wayland protocols directory
pkg_get_variable(WAYLAND_PROTOCOLS_DIR wayland-protocols pkgdatadir)

# Create directory for generated files
set(GENERATED_DIR ${CMAKE_BINARY_DIR}/generated)
file(MAKE_DIRECTORY ${GENERATED_DIR})

# Find all XML protocol files
file(GLOB PROTOCOL_FILES protocols/*.xml)

# Also add the XDG shell protocol from wayland-protocols
list(APPEND PROTOCOL_FILES ${WAYLAND_PROTOCOLS_DIR}/stable/xdg-shell/xdg-shell.xml)

# Generate protocol sources and headers
set(PROTOCOL_SOURCES "")
set(PROTOCOL_HEADERS "")

foreach(PROTOCOL_FILE ${PROTOCOL_FILES})
    get_filename_component(PROTOCOL_NAME ${PROTOCOL_FILE} NAME_WE)

    # Generate header file
    set(PROTOCOL_HEADER ${GENERATED_DIR}/${PROTOCOL_NAME}-client-protocol.h)
    add_custom_command(
        OUTPUT ${PROTOCOL_HEADER}
        COMMAND ${WAYLAND_SCANNER} client-header ${PROTOCOL_FILE} ${PROTOCOL_HEADER}
        DEPENDS ${PROTOCOL_FILE}
        COMMENT "Generating ${PROTOCOL_NAME} header"
    )
    list(APPEND PROTOCOL_HEADERS ${PROTOCOL_HEADER})

    # Generate source file
    set(PROTOCOL_SOURCE ${GENERATED_DIR}/${PROTOCOL_NAME}-client-protocol.c)
    add_custom_command(
        OUTPUT ${PROTOCOL_SOURCE}
        COMMAND ${WAYLAND_SCANNER} private-code ${PROTOCOL_FILE} ${PROTOCOL_SOURCE}
        DEPENDS ${PROTOCOL_FILE}
        COMMENT "Generating ${PROTOCOL_NAME} source"
    )
    list(APPEND PROTOCOL_SOURCES ${PROTOCOL_SOURCE})
endforeach()

# Find C++ source files only
file(GLOB_RECURSE CPP_SOURCE_FILES src/*.cpp)

# Create executable
add_executable(${PROJECT_NAME} 
    ${CPP_SOURCE_FILES}
    ${PROTOCOL_SOURCES}
)

# Set source file properties to compile C files as C
set_source_files_properties(${PROTOCOL_SOURCES} PROPERTIES LANGUAGE C)

# Include directories
target_include_directories(${PROJECT_NAME} PRIVATE
    ${GENERATED_DIR}
    ${WAYLAND_CLIENT_INCLUDE_DIRS}
    src
)

# Link libraries
target_link_libraries(${PROJECT_NAME} PRIVATE
    ${WAYLAND_CLIENT_LIBRARIES}
    wayland-client
)

# Compiler flags
target_compile_options(${PROJECT_NAME} PRIVATE
    ${WAYLAND_CLIENT_CFLAGS_OTHER}
)

# Ensure protocol files are generated before compiling source
add_custom_target(generate_protocols ALL
    DEPENDS ${PROTOCOL_HEADERS} ${PROTOCOL_SOURCES}
)
add_dependencies(${PROJECT_NAME} generate_protocols)

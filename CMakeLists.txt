cmake_minimum_required(VERSION 3.15)

project(
    WaypaperEngine
    DESCRIPTION "A customizable, animated wallpaper inspired from wallpaper engine"
    VERSION 0.0.1
    LANGUAGES CXX
)

set(CMAKE_CXX_STANDARD 23)
add_compile_options(-Wall -Wextra -Werror)


file(GLOB_RECURSE SRCFILES "src/*.cpp")
include_directories(.)
add_executable(waypaper-engine ${SRCFILES})
target_link_libraries(waypaper-engine PRIVATE wayland-client)

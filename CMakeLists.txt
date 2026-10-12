cmake_minimum_required(VERSION 3.16)
project(CMA_Chemistry VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

add_compile_definitions(_USE_MATH_DEFINES)

# ── Dependencias externas (usando directorios cacheados) ──
include(FetchContent)

set(FETCHCONTENT_BASE_DIR ${CMAKE_BINARY_DIR}/_deps)

FetchContent_Declare(glfw
    GIT_REPOSITORY https://github.com/glfw/glfw.git
    GIT_TAG 3.4
    SOURCE_DIR ${CMAKE_SOURCE_DIR}/deps_win/glfw_src
)
FetchContent_Declare(imgui
    GIT_REPOSITORY https://github.com/ocornut/imgui.git
    GIT_TAG v1.91.8
    SOURCE_DIR ${FETCHCONTENT_BASE_DIR}/imgui-src
)
FetchContent_Declare(implot
    GIT_REPOSITORY https://github.com/epezent/implot.git
    GIT_TAG v0.17
    SOURCE_DIR ${FETCHCONTENT_BASE_DIR}/implot-src
)
FetchContent_Declare(eigen
    GIT_REPOSITORY https://gitlab.com/libeigen/eigen.git
    GIT_TAG 3.4.0
    SOURCE_DIR ${FETCHCONTENT_BASE_DIR}/eigen-src
)
FetchContent_Declare(implot3d
    GIT_REPOSITORY https://github.com/brenocq/implot3d.git
    GIT_TAG main
    SOURCE_DIR ${FETCHCONTENT_BASE_DIR}/implot3d-src
)

set(FETCHCONTENT_UPDATES_DISCONNECTED ON)

set(GLFW_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
set(GLFW_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(GLFW_BUILD_DOCS OFF CACHE BOOL "" FORCE)
set(GLFW_INSTALL OFF CACHE BOOL "" FORCE)

FetchContent_MakeAvailable(glfw imgui implot implot3d eigen)

find_package(SQLite3 REQUIRED)

# ── Módulos nuevos ──
add_subdirectory(src/sim)
add_subdirectory(src/thermo)
add_subdirectory(src/diagrams)
add_subdirectory(src/aqueous)
add_subdirectory(src/db)
add_subdirectory(src/estimation)
add_subdirectory(src/tools)

# ── Fuentes de ImGui ──
set(IMGUI_SOURCES
    ${imgui_SOURCE_DIR}/imgui.cpp
    ${imgui_SOURCE_DIR}/imgui_draw.cpp
    ${imgui_SOURCE_DIR}/imgui_tables.cpp
    ${imgui_SOURCE_DIR}/imgui_widgets.cpp
    ${imgui_SOURCE_DIR}/backends/imgui_impl_glfw.cpp
    ${imgui_SOURCE_DIR}/backends/imgui_impl_opengl3.cpp
)
set(IMPLOT_SOURCES
    ${implot_SOURCE_DIR}/implot.cpp
    ${implot_SOURCE_DIR}/implot_items.cpp
)
set(IMPLOT3D_SOURCES
    ${implot3d_SOURCE_DIR}/implot3d.cpp
    ${implot3d_SOURCE_DIR}/implot3d_items.cpp
    ${implot3d_SOURCE_DIR}/implot3d_meshes.cpp
)

# ── Ejecutable principal ──
add_executable(cma_chemistry
    src/main.cpp
    src/nucleo/tabla_periodica.cpp
    src/base_datos/gestor_db.cpp
    src/base_datos/gestor_elementos.cpp
    src/base_datos/gestor_unidades.cpp
    src/diagrama_flujo/flowsheet.cpp
    src/interfaz/aplicacion.cpp
    src/interfaz/paneles.cpp
    src/interfaz/lienzo.cpp
    src/motores/balance_masa.cpp
    src/motores/balance_energia.cpp
    src/motores/balance_exergia.cpp
    src/motores/minimizador_gibbs.cpp
    src/motores/equilibrio_fases.cpp
    src/motores/electroquimica.cpp
    src/motores/cinetica.cpp
    src/motores/destilacion.cpp
    src/motores/simulacion_dinamica.cpp
    src/motores/reconciliacion_datos.cpp
    src/motores/exergia.cpp
    src/motores/agua.cpp
    src/motores/soluciones_acuosas.cpp
    src/motores/perdida_calor.cpp
    src/motores/estimador_propiedades.cpp
    src/motores/lca.cpp
    src/motores/pourbaix.cpp
    src/utils/i18n.cpp
    ${IMGUI_SOURCES}
    ${IMPLOT_SOURCES}
    ${IMPLOT3D_SOURCES}
)
target_include_directories(cma_chemistry PRIVATE
    src
    ${glfw_SOURCE_DIR}/include
    ${imgui_SOURCE_DIR}
    ${imgui_SOURCE_DIR}/backends
    ${implot_SOURCE_DIR}
    ${implot3d_SOURCE_DIR}
    ${eigen_SOURCE_DIR}
    ${SQLite3_INCLUDE_DIRS}
)
target_link_libraries(cma_chemistry PRIVATE
    glfw ${CMAKE_DL_LIBS} ${SQLite3_LIBRARIES}
    cma_sim cma_thermo cma_diagrams cma_aqueous
    cma_db cma_estimation cma_tools
)

find_package(OpenGL REQUIRED)
target_link_libraries(cma_chemistry PRIVATE OpenGL::GL)

# ── Instalación y Empaquetado (CPack) ──
install(TARGETS cma_chemistry DESTINATION bin)
# Si hay bases de datos, deberían copiarse a bin o share (por simplicidad, asumimos relativas al binario en dev)

set(CPACK_PACKAGE_NAME "CMA_Chemistry")
set(CPACK_PACKAGE_VENDOR "CMA")
set(CPACK_PACKAGE_VERSION "1.0.0")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "CMA Chemistry - Chemical Engineering Simulator")

if(WIN32)
    set(CPACK_GENERATOR "NSIS")
    set(CPACK_NSIS_DISPLAY_NAME "CMA Chemistry")
else()
    set(CPACK_GENERATOR "DEB;TGZ")
    set(CPACK_DEBIAN_PACKAGE_MAINTAINER "Nando")
    set(CPACK_DEBIAN_PACKAGE_DEPENDS "libgl1, libsqlite3-0")
endif()

include(CPack)

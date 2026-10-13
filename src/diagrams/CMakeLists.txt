add_library(cma_diagrams STATIC
    PropertyDiagrams.cpp
    StabilityDiagrams.cpp
)
target_include_directories(cma_diagrams PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_include_directories(cma_diagrams PRIVATE ${CMAKE_SOURCE_DIR}/src)
target_link_libraries(cma_diagrams PUBLIC cma_domain)

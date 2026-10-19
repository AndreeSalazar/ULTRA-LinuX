cmake_minimum_required(VERSION 3.30)
project(UltraInvestigacionCXX LANGUAGES CXX)

if(NOT CMAKE_CXX_COMPILER_ID STREQUAL "GNU" OR CMAKE_CXX_COMPILER_VERSION VERSION_LESS 16)
    message(FATAL_ERROR "Este primer perfil requiere G++ >= 16; otros compiladores necesitan un perfil propio")
endif()
set(CMAKE_CXX_STANDARD 26)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

option(ULTRA_CXX_DIAGNOSTICO "ASan/UBSan para el banco CPU; no usar para medir rendimiento" OFF)
add_subdirectory(produccion)
add_subdirectory(laboratorio)

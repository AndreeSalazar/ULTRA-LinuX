# Solo para fuentes propias de este banco. No inyectar en Hyprland ni dependencias.
add_library(ultra_cxx_perfil INTERFACE)
target_compile_options(ultra_cxx_perfil INTERFACE
    -Wall -Wextra -Wpedantic -Werror -Wconversion -Wsign-conversion -Wshadow
    -Wformat=2 -Wold-style-cast -Wnon-virtual-dtor -Woverloaded-virtual
    -fno-lto -fhardened)
target_link_options(ultra_cxx_perfil INTERFACE -fhardened -fno-lto)
if(ULTRA_CXX_DIAGNOSTICO)
    target_compile_definitions(ultra_cxx_perfil INTERFACE ULTRA_CXX_DIAGNOSTICO=1)
    target_compile_options(ultra_cxx_perfil INTERFACE
        -O1 -g -fno-omit-frame-pointer -fsanitize=address,undefined
        -fno-sanitize-recover=all)
    target_link_options(ultra_cxx_perfil INTERFACE
        -fsanitize=address,undefined -fno-sanitize-recover=all)
else()
    target_compile_options(ultra_cxx_perfil INTERFACE -O2)
endif()

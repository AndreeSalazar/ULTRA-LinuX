// SPDX-License-Identifier: GPL-2.0-only
#include "../../produccion/gpus.hpp"
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES3/gl3.h>
#include <array>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>
#include <utility>

namespace {
using Bytes = std::vector<std::uint8_t>;
using ultra::gpus::Mode;
void require(bool ok, const char* message) { if (!ok) throw std::runtime_error(message); }
struct Session {
    EGLDisplay display = EGL_NO_DISPLAY;
    EGLSurface surface = EGL_NO_SURFACE;
    EGLContext context = EGL_NO_CONTEXT;
    Session() = default;
    Session(const Session&) = delete;
    Session& operator=(const Session&) = delete;
    void open() {
        display = eglGetPlatformDisplay(EGL_PLATFORM_SURFACELESS_MESA, EGL_DEFAULT_DISPLAY, nullptr);
        require(display != EGL_NO_DISPLAY && eglInitialize(display, nullptr, nullptr), "EGL init");
        require(eglBindAPI(EGL_OPENGL_ES_API), "EGL bind");
        const EGLint attrs[] = {EGL_SURFACE_TYPE, EGL_PBUFFER_BIT, EGL_RENDERABLE_TYPE, EGL_OPENGL_ES3_BIT,
            EGL_RED_SIZE, 8, EGL_GREEN_SIZE, 8, EGL_BLUE_SIZE, 8, EGL_ALPHA_SIZE, 8, EGL_NONE};
        EGLConfig config{};
        EGLint count = 0;
        require(eglChooseConfig(display, attrs, &config, 1, &count) && count > 0, "EGL config");
        const EGLint sa[] = {EGL_WIDTH, 8, EGL_HEIGHT, 8, EGL_NONE};
        surface = eglCreatePbufferSurface(display, config, sa);
        const EGLint ca[] = {EGL_CONTEXT_CLIENT_VERSION, 3, EGL_NONE};
        context = eglCreateContext(display, config, EGL_NO_CONTEXT, ca);
        require(surface != EGL_NO_SURFACE && context != EGL_NO_CONTEXT, "EGL resources");
        require(eglMakeCurrent(display, surface, surface, context), "EGL current");
    }
    ~Session() {
        if (display != EGL_NO_DISPLAY) {
            eglMakeCurrent(display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
            if (context != EGL_NO_CONTEXT) eglDestroyContext(display, context);
            if (surface != EGL_NO_SURFACE) eglDestroySurface(display, surface);
            eglTerminate(display);
        }
        eglReleaseThread();
    }
};
struct Program {
    GLuint id = glCreateProgram();
    Program() { require(id != 0, "glCreateProgram"); }
    Program(const Program&) = delete;
    Program& operator=(const Program&) = delete;
    ~Program() { glDeleteProgram(id); }
};
struct Shader {
    GLuint id;
    explicit Shader(GLenum type) : id(glCreateShader(type)) { require(id != 0, "glCreateShader"); }
    Shader(const Shader&) = delete;
    Shader& operator=(const Shader&) = delete;
    ~Shader() { glDeleteShader(id); }
};
constexpr const char* vertex = "#version 300 es\nvoid main(){\n"
    "vec2 p[3]=vec2[3](vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));\n"
    "gl_Position=vec4(p[gl_VertexID],0.0,1.0);}\n";
constexpr const char* fragment = "#version 300 es\nprecision highp float;\nout vec4 color;\n"
    "void main(){color=vec4(0.0,1.0,0.0,1.0);}\n";
std::string identity() {
    std::string key = "ULTRA-GPUS-v1;GLES3;retrievable=1;shader-options=default\n";
    for (GLenum name : std::array<GLenum, 4>{GL_VENDOR, GL_RENDERER, GL_VERSION, GL_SHADING_LANGUAGE_VERSION}) {
        const auto* value = glGetString(name);
        require(value != nullptr, "GL identity");
        key += reinterpret_cast<const char*>(value);
        key += '\n';
    }
    require(key.find("llvmpipe") != std::string::npos, "laboratorio solo software");
    key += vertex;
    key += fragment;
    return key;
}
bool try_compile(GLuint program, const char* fragment_source) {
    Shader vs{GL_VERTEX_SHADER}, fs{GL_FRAGMENT_SHADER};
    for (const auto& item : {std::pair{vs.id, vertex}, std::pair{fs.id, fragment_source}}) {
        glShaderSource(item.first, 1, &item.second, nullptr);
        glCompileShader(item.first);
        GLint ok = 0;
        glGetShaderiv(item.first, GL_COMPILE_STATUS, &ok);
        if (ok != GL_TRUE) return false;
        glAttachShader(program, item.first);
    }
    glProgramParameteri(program, GL_PROGRAM_BINARY_RETRIEVABLE_HINT, GL_TRUE);
    glLinkProgram(program);
    GLint linked = 0;
    glGetProgramiv(program, GL_LINK_STATUS, &linked);
    const auto error = glGetError();
    glDetachShader(program, vs.id);
    glDetachShader(program, fs.id);
    return linked == GL_TRUE && error == GL_NO_ERROR;
}
void compile(GLuint program) {
    require(try_compile(program, fragment), "compilacion/link del programa de confianza");
}
Bytes save(GLuint program, const std::string& key) {
    GLint size = 0;
    glGetProgramiv(program, GL_PROGRAM_BINARY_LENGTH, &size);
    require(size > 0 && static_cast<std::size_t>(size) <= ultra::gpus::max_binary, "binario no disponible o excesivo");
    Bytes binary(static_cast<std::size_t>(size));
    GLsizei written = 0;
    GLenum format = 0;
    glGetProgramBinary(program, size, &written, &format, binary.data());
    require(glGetError() == GL_NO_ERROR && written > 0 && written <= size, "extraer binario");
    binary.resize(static_cast<std::size_t>(written));
    auto encoded = ultra::gpus::encode(format, key, binary);
    require(encoded.has_value(), "encode gpus");
    return std::move(*encoded);
}
bool import(GLuint program, const Bytes& data, const std::string& key) {
    const auto view = ultra::gpus::parse(data, key);
    if (!view) return false;
    GLint count = 0;
    glGetIntegerv(GL_NUM_PROGRAM_BINARY_FORMATS, &count);
    if (count <= 0 || count > 256) return false;
    std::vector<GLint> formats(static_cast<std::size_t>(count));
    glGetIntegerv(GL_PROGRAM_BINARY_FORMATS, formats.data());
    bool supported = false;
    for (GLint format : formats) if (static_cast<GLenum>(format) == view->format) supported = true;
    if (!supported) return false;
    glProgramBinary(program, view->format, view->binary.data(), static_cast<GLsizei>(view->binary.size()));
    const GLenum error = glGetError();
    GLint linked = 0;
    glGetProgramiv(program, GL_LINK_STATUS, &linked);
    return error == GL_NO_ERROR && linked == GL_TRUE && glGetError() == GL_NO_ERROR;
}
void draw(GLuint program, const std::array<GLubyte, 4>& expected = {0, 255, 0, 255}) {
    glViewport(0, 0, 8, 8);
    glClearColor(1.0F, 0.0F, 0.0F, 1.0F);
    glClear(GL_COLOR_BUFFER_BIT);
    glUseProgram(program);
    glDrawArrays(GL_TRIANGLES, 0, 3);
    std::array<GLubyte, 4> pixel{};
    glReadPixels(4, 4, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, pixel.data());
    require(glGetError() == GL_NO_ERROR, "draw/read GL");
    require(pixel == expected, "shader no produjo el pixel esperado");
    glUseProgram(0);
}
struct Counters { unsigned compiled = 0, imported = 0; };
bool load(GLuint program, Bytes& data, const std::string& key, Mode mode, Counters& counts) {
    if (import(program, data, key)) { ++counts.imported; return true; }
    if (mode == Mode::static_only) return false;
    compile(program); // Only embedded trusted source; never source from the cache.
    ++counts.compiled;
    data = save(program, key);
    return true;
}
void file_roundtrip(Bytes& data) {
    const auto size = static_cast<std::streamsize>(data.size());
    {
        std::ofstream out{"/tmp/control.gpus", std::ios::binary | std::ios::trunc};
        out.write(reinterpret_cast<const char*>(data.data()), size);
        require(out.good(), "escribir .gpus");
    }
    std::ifstream in{"/tmp/control.gpus", std::ios::binary | std::ios::ate};
    require(in.good() && in.tellg() == size, "tamano de archivo .gpus");
    Bytes read(data.size());
    in.seekg(0);
    in.read(reinterpret_cast<char*>(read.data()), size);
    require(in.good() && read == data, "leer .gpus");
    data = std::move(read);
}
}

int main() {
    try {
        Bytes cache;
        std::string key;
        {
            Session session; session.open();
            key = identity();
            Program program;
            Counters counts;
            require(load(program.id, cache, key, Mode::dynamic_rebuild, counts), "cache inicial");
            require(counts.compiled == 1 && counts.imported == 0, "dinamico sin cache no compilo");
            draw(program.id);
            file_roundtrip(cache);
            std::cout << "VERDE [S-01]: dinamico compila, dibuja y guarda .gpus (" << cache.size() << " bytes)\n";
        }
        // Destroyed first context; import into a new context in the same process.
        Session session; session.open();
        require(identity() == key, "identidad cambio entre contextos");
        {
            Program program; Counters counts;
            require(load(program.id, cache, key, Mode::static_only, counts), "estatico no importa");
            require(counts.compiled == 0 && counts.imported == 1, "estatico recompilo");
            draw(program.id);
            std::cout << "VERDE [S-02]: estatico importa en contexto nuevo y produce el mismo pixel sin recompilar GLSL\n";
        }
        {
            Program program; Counters counts;
            require(load(program.id, cache, key, Mode::dynamic_rebuild, counts), "dinamico hit");
            require(counts.compiled == 0 && counts.imported == 1, "hit dinamico recompilo");
            draw(program.id);
            std::cout << "VERDE [S-03]: dinamico reutiliza cache compatible\n";
        }
        for (unsigned test = 0; test < 10; ++test) {
            Bytes broken = cache;
            std::string requested = key;
            if (test == 0) broken.clear();
            if (test == 1) broken.resize(10);
            if (test == 2) broken[0] ^= 1;
            if (test == 3) ultra::gpus::write32(broken, 4, 2);
            if (test == 4) ultra::gpus::write32(broken, 16, 0xFFFFFFFFU);
            if (test == 5) broken.back() ^= 1;
            if (test == 6) requested += "changed-source-or-driver";
            if (test == 7) {
                ultra::gpus::write32(broken, 8, 0xFFFFFFFFU);
                ultra::gpus::write32(broken, 20, ultra::gpus::checksum(broken));
            }
            if (test == 8) broken.push_back(0);
            if (test == 9) ultra::gpus::write32(broken, 16, 0);
            {
                Program strict; Counters counts;
                require(!load(strict.id, broken, requested, Mode::static_only, counts), "estatico acepto cache invalida");
                require(counts.compiled == 0 && counts.imported == 0, "estatico cambio estado");
            }
            Program repaired; Counters counts;
            require(load(repaired.id, broken, requested, Mode::dynamic_rebuild, counts), "dinamico no reconstruye");
            require(counts.compiled == 1 && counts.imported == 0, "dinamico no conto reconstruccion");
            draw(repaired.id);
            require(ultra::gpus::parse(broken, requested).has_value(), "cache regenerada invalida");
            std::cout << "VERDE [S-04." << test << "]: estatico rechaza y dinamico reconstruye caso invalido\n";
        }
        {
            Program active;
            compile(active.id); draw(active.id);
            {
                Program candidate;
                require(!try_compile(candidate.id, "#version 300 es\nshader roto\n"), "reemplazo invalido compilo");
                draw(active.id);
            }
            std::cout << "VERDE [S-05]: reemplazo fallido conserva programa y pixel anteriores\n";
            {
                Program candidate;
                require(try_compile(candidate.id, "#version 300 es\nprecision highp float;\nout vec4 color;\n"
                    "void main(){color=vec4(0.0,0.0,1.0,1.0);}\n"), "reemplazo valido no compila");
                draw(candidate.id, {0, 0, 255, 255});
                draw(active.id); // Previous frame completed via glReadPixels before committing.
                std::swap(active.id, candidate.id);
            }
            draw(active.id, {0, 0, 255, 255});
            std::cout << "VERDE [S-06]: candidato validado se activa entre dibujos y programa anterior se libera\n";
        }
        std::cout << "ALCANCE: GLES experimental; no hot-reload asincrono ni aceleracion medida\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "ROJO: " << error.what() << '\n';
        return 1;
    }
}

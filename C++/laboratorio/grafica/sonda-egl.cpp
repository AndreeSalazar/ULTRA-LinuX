// SPDX-License-Identifier: GPL-2.0-only
// Small, bounded GLES3 API tests. No deliberate C++ UB or GPU exhaustion.
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES3/gl3.h>
#include <array>
#include <iostream>
#include <stdexcept>
#include <string>
#include <string_view>
#include "../../produccion/guard_gles.hpp"

namespace {
void require(bool condition, const char* message) {
    if (!condition) throw std::runtime_error(message);
}
const char* text(const GLubyte* value) {
    require(value != nullptr, "glGetString devolvio null");
    return reinterpret_cast<const char*>(value);
}
struct Session {
    EGLDisplay display = EGL_NO_DISPLAY;
    EGLSurface surface = EGL_NO_SURFACE;
    EGLContext context = EGL_NO_CONTEXT;
    Session() = default;
    Session(const Session&) = delete;
    Session& operator=(const Session&) = delete;
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
struct Shader {
    GLuint id;
    explicit Shader(GLenum type) : id(glCreateShader(type)) { require(id != 0, "glCreateShader"); }
    Shader(const Shader&) = delete;
    Shader& operator=(const Shader&) = delete;
    ~Shader() { glDeleteShader(id); }
    bool compile(const char* source) const {
        glShaderSource(id, 1, &source, nullptr);
        glCompileShader(id);
        GLint status = GL_FALSE;
        glGetShaderiv(id, GL_COMPILE_STATUS, &status);
        return status == GL_TRUE;
    }
};
void pixel_control() {
    glViewport(0, 0, 8, 8);
    glClearColor(0.0F, 1.0F, 0.0F, 1.0F);
    glClear(GL_COLOR_BUFFER_BIT);
    std::array<GLubyte, 4> pixel{};
    glReadPixels(4, 4, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, pixel.data());
    require(glGetError() == GL_NO_ERROR, "clear/read produjo error GL");
    require(pixel[0] == 0 && pixel[1] == 255 && pixel[2] == 0 && pixel[3] == 255,
            "pixel no coincide con verde RGBA8");
}
void texture_controls(Session& session, EGLConfig config) {
    std::array<std::uint8_t, 64> pixels{};
    for (std::size_t i = 0; i < pixels.size(); i += 4) {
        pixels[i + 1] = 255; pixels[i + 3] = 255;
    }
    require(!ultra::gles::Texture::rgba8(0, 4, pixels, 64), "dimension cero aceptada");
    require(!ultra::gles::Texture::rgba8(-1, 4, pixels, 64), "dimension negativa aceptada");
    require(!ultra::gles::Texture::rgba8(4, 4, std::span{pixels}.first(63), 64), "capacidad insuficiente aceptada");
    require(!ultra::gles::Texture::rgba8(4, 4, pixels, 63), "presupuesto excedido aceptado");
    GLint max_dimension = 0;
    glGetIntegerv(GL_MAX_TEXTURE_SIZE, &max_dimension);
    require(!ultra::gles::Texture::rgba8(std::int64_t{max_dimension} + 1, 1, pixels, 64), "limite GPU ignorado");
    std::cout << "VERDE [G-06]: dimensiones, capacidad, presupuesto y limite del dispositivo rechazados\n";

    GLuint sentinel = 0, pbo = 0;
    glGenTextures(1, &sentinel); glBindTexture(GL_TEXTURE_2D, sentinel);
    glGenBuffers(1, &pbo); glBindBuffer(GL_PIXEL_UNPACK_BUFFER, pbo);
    glBufferData(GL_PIXEL_UNPACK_BUFFER, 64, nullptr, GL_STATIC_DRAW);
    glPixelStorei(GL_UNPACK_ALIGNMENT, 8);
    glPixelStorei(GL_UNPACK_ROW_LENGTH, 16);
    glPixelStorei(GL_UNPACK_SKIP_ROWS, 2);
    glPixelStorei(GL_UNPACK_SKIP_PIXELS, 2);
    require(glGetError() == GL_NO_ERROR, "preparar estado de carga adverso");
    auto texture = ultra::gles::Texture::rgba8(4, 4, pixels, 64);
    require(texture.has_value(), "textura valida rechazada");
    GLint value = 0;
    glGetIntegerv(GL_TEXTURE_BINDING_2D, &value);
    require(static_cast<GLuint>(value) == sentinel, "binding textura no restaurado");
    glGetIntegerv(GL_PIXEL_UNPACK_BUFFER_BINDING, &value);
    require(static_cast<GLuint>(value) == pbo, "binding PBO no restaurado");
    for (const auto& entry : {std::pair{GL_UNPACK_ALIGNMENT, 8}, std::pair{GL_UNPACK_ROW_LENGTH, 16},
                            std::pair{GL_UNPACK_SKIP_ROWS, 2}, std::pair{GL_UNPACK_SKIP_PIXELS, 2}}) {
        glGetIntegerv(static_cast<GLenum>(entry.first), &value);
        require(value == entry.second, "pixel store no restaurado");
    }
    GLuint fbo = 0;
    glGenFramebuffers(1, &fbo); glBindFramebuffer(GL_FRAMEBUFFER, fbo);
    glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, texture->name(), 0);
    require(glCheckFramebufferStatus(GL_FRAMEBUFFER) == GL_FRAMEBUFFER_COMPLETE, "textura incompleta");
    std::array<GLubyte, 4> pixel{};
    glReadPixels(1, 1, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, pixel.data());
    require(glGetError() == GL_NO_ERROR && pixel == std::array<GLubyte, 4>{0,255,0,255}, "pixel de textura incorrecto");
    const GLuint stable_name = texture->name();
    for (unsigned cycle = 0; cycle < 64; ++cycle) {
        for (std::size_t i = 0; i < pixels.size(); i += 4)
            pixels[i] = static_cast<std::uint8_t>(cycle);
        require(texture->update_rgba8(pixels, 64).has_value(), "actualizacion de textura");
        require(texture->name() == stable_name, "actualizacion recreo nombre");
        glReadPixels(1, 1, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, pixel.data());
        require(pixel == std::array<GLubyte, 4>{static_cast<GLubyte>(cycle),255,0,255},
                "actualizacion no llego al pixel");
    }
    const auto small = texture->update_rgba8(std::span{pixels}.first(63), 64);
    const auto budget = texture->update_rgba8(pixels, 63);
    require(!small && small.error() == ultra::gles::Error::layout &&
            !budget && budget.error() == ultra::gles::Error::layout, "update acepto limites invalidos");
    glEnable(0xFFFFFFFFU);
    const auto dirty_update = texture->update_rgba8(pixels, 64);
    require(!dirty_update && dirty_update.error() == ultra::gles::Error::dirty_state,
            "update ignoro error anterior");
    glReadPixels(1, 1, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, pixel.data());
    require(pixel == std::array<GLubyte, 4>{63,255,0,255}, "rechazo altero contenido");
    glGetIntegerv(GL_TEXTURE_BINDING_2D, &value);
    require(static_cast<GLuint>(value) == sentinel, "update altero binding textura");
    glGetIntegerv(GL_PIXEL_UNPACK_BUFFER_BINDING, &value);
    require(static_cast<GLuint>(value) == pbo, "update altero binding PBO");
    for (const auto& entry : {std::pair{GL_UNPACK_ALIGNMENT, 8}, std::pair{GL_UNPACK_ROW_LENGTH, 16},
                            std::pair{GL_UNPACK_SKIP_ROWS, 2}, std::pair{GL_UNPACK_SKIP_PIXELS, 2}}) {
        glGetIntegerv(static_cast<GLenum>(entry.first), &value);
        require(value == entry.second, "update altero pixel store");
    }
    require(glGetError() == GL_NO_ERROR, "update produjo error GL");
    // Restore the original test data for subsequent controls.
    for (std::size_t i = 0; i < pixels.size(); i += 4) pixels[i] = 0;
    std::cout << "VERDE [G-12]: 64 actualizaciones sin recrear textura; pixels, limites y estado conservados\n";
    glBindFramebuffer(GL_FRAMEBUFFER, 0); glDeleteFramebuffers(1, &fbo);
    std::cout << "VERDE [G-07]: textura correcta con PBO/stride adversos y estado restaurado\n";

    require(eglMakeCurrent(session.display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT), "desvincular para control");
    const auto closed_without_owner = texture->close();
    require(!closed_without_owner && closed_without_owner.error() == ultra::gles::Error::wrong_context,
            "cierre sin contexto propietario aceptado");
    const auto update_without_owner = texture->update_rgba8(pixels, 64);
    require(!update_without_owner && update_without_owner.error() == ultra::gles::Error::wrong_context,
            "update sin contexto propietario aceptado");
    const auto without_context = ultra::gles::Texture::rgba8(4, 4, pixels, 64);
    require(!without_context && without_context.error() == ultra::gles::Error::no_context, "creacion sin contexto aceptada");
    require(eglMakeCurrent(session.display, session.surface, session.surface, session.context), "restaurar contexto");
    const GLuint name = texture->name();
    require(glIsTexture(name) == GL_TRUE, "recurso desaparecio antes del cierre");
    require(texture->close().has_value() && glIsTexture(name) == GL_FALSE, "recurso no liberado");
    require(texture->close().has_value(), "segundo close no es idempotente");
    const auto after_close = texture->update_rgba8(pixels, 64);
    require(!after_close && after_close.error() == ultra::gles::Error::closed,
            "update despues de close aceptado");
    std::cout << "VERDE [G-08]: propiedad de contexto, rechazo sin contexto y cierre idempotente\n";
    glBindBuffer(GL_PIXEL_UNPACK_BUFFER, 0); glDeleteBuffers(1, &pbo);
    glBindTexture(GL_TEXTURE_2D, 0); glDeleteTextures(1, &sentinel);
    glPixelStorei(GL_UNPACK_ALIGNMENT, 4); glPixelStorei(GL_UNPACK_ROW_LENGTH, 0);
    glPixelStorei(GL_UNPACK_SKIP_ROWS, 0); glPixelStorei(GL_UNPACK_SKIP_PIXELS, 0);
    GLuint scoped_name = 0;
    {
        auto scoped = ultra::gles::Texture::rgba8(4, 4, pixels, 64);
        require(scoped.has_value(), "textura de ambito");
        scoped_name = scoped->name();
    }
    require(glIsTexture(scoped_name) == GL_FALSE && glGetError() == GL_NO_ERROR, "RAII no libero textura");
    glEnable(0xFFFFFFFFU);
    const auto dirty = ultra::gles::Texture::rgba8(4, 4, pixels, 64);
    require(!dirty && dirty.error() == ultra::gles::Error::dirty_state, "error previo ignorado");
    pixel_control();
    std::cout << "VERDE [G-09]: RAII libera y error GL previo se rechaza sin romper contexto\n";

    // An unrelated context may reuse the same numeric name: close must not touch it.
    auto owned = ultra::gles::Texture::rgba8(4, 4, pixels, 64);
    require(owned.has_value(), "textura antes de cambiar contexto");
    {
        struct OtherContext {
            Session& owner;
            EGLContext other;
            OtherContext(Session& session_owner, EGLConfig configuration) : owner(session_owner) {
                const EGLint attributes[] = {EGL_CONTEXT_CLIENT_VERSION, 3, EGL_NONE};
                other = eglCreateContext(owner.display, configuration, EGL_NO_CONTEXT, attributes);
                require(other != EGL_NO_CONTEXT, "crear segundo contexto");
            }
            ~OtherContext() {
                // Restore even when an assertion unwinds, before Texture destruction.
                if (!eglMakeCurrent(owner.display, owner.surface, owner.surface, owner.context))
                    std::terminate();
                eglDestroyContext(owner.display, other);
            }
        } alternate{session, config};
        require(eglMakeCurrent(session.display, session.surface, session.surface, alternate.other),
                "activar segundo contexto");
        const auto rejected = owned->close();
        require(!rejected && rejected.error() == ultra::gles::Error::wrong_context,
                "cierre en otro contexto aceptado");
        const auto bad_update = owned->update_rgba8(pixels, 64);
        require(!bad_update && bad_update.error() == ultra::gles::Error::wrong_context,
                "update en otro contexto aceptado");
    }
    require(glIsTexture(owned->name()) == GL_TRUE && owned->close().has_value(),
            "rechazo en segundo contexto altero propietario");
    std::cout << "VERDE [G-10]: otro contexto rechazado; recurso del propietario conservado\n";

    for (unsigned cycle = 0; cycle < 256; ++cycle) {
        GLuint released = 0;
        {
            auto original = ultra::gles::Texture::rgba8(4, 4, pixels, 64);
            require(original.has_value(), "ciclo de textura");
            released = original->name();
            ultra::gles::Texture moved{std::move(*original)};
            require(original->name() == 0 && moved.name() == released,
                    "move no transfirio propiedad unica");
            require(original->close().has_value() && glIsTexture(released) == GL_TRUE,
                    "objeto movido libero recurso del nuevo propietario");
            const auto moved_update = original->update_rgba8(pixels, 64);
            require(!moved_update && moved_update.error() == ultra::gles::Error::closed,
                    "update sobre objeto movido aceptado");
        }
        require(glIsTexture(released) == GL_FALSE && glGetError() == GL_NO_ERROR,
                "ciclo dejo nombre de textura vivo o error GL");
    }
    pixel_control();
    std::cout << "VERDE [G-11]: 256 ciclos crear/mover/destruir; nombres liberados y pixel posterior correcto\n";
}
}

int main() {
    try {
        // bwrap runner supplies no GPU, network or session sockets. Verify the GPU path too.
        Session session;
        session.display = eglGetPlatformDisplay(EGL_PLATFORM_SURFACELESS_MESA, EGL_DEFAULT_DISPLAY, nullptr);
        require(session.display != EGL_NO_DISPLAY, "EGL surfaceless no disponible");
        EGLint major = 0, minor = 0;
        require(eglInitialize(session.display, &major, &minor) == EGL_TRUE, "eglInitialize");
        const char* vendor = eglQueryString(session.display, EGL_VENDOR);
        require(vendor != nullptr, "EGL_VENDOR ausente");
        std::cout << "EGL: " << major << '.' << minor << " vendor=" << vendor << '\n';
        require(eglBindAPI(EGL_OPENGL_ES_API) == EGL_TRUE, "eglBindAPI GLES");
        const EGLint attrs[] = {EGL_SURFACE_TYPE, EGL_PBUFFER_BIT, EGL_RENDERABLE_TYPE,
            EGL_OPENGL_ES3_BIT, EGL_RED_SIZE, 8, EGL_GREEN_SIZE, 8, EGL_BLUE_SIZE, 8,
            EGL_ALPHA_SIZE, 8, EGL_NONE};
        EGLConfig config{};
        EGLint count = 0;
        require(eglChooseConfig(session.display, attrs, &config, 1, &count) == EGL_TRUE && count > 0,
                "config GLES3/RGBA8 ausente");
        const EGLint surface_attrs[] = {EGL_WIDTH, 8, EGL_HEIGHT, 8, EGL_NONE};
        session.surface = eglCreatePbufferSurface(session.display, config, surface_attrs);
        require(session.surface != EGL_NO_SURFACE, "eglCreatePbufferSurface");
        const EGLint context_attrs[] = {EGL_CONTEXT_CLIENT_VERSION, 3, EGL_NONE};
        session.context = eglCreateContext(session.display, config, EGL_NO_CONTEXT, context_attrs);
        require(session.context != EGL_NO_CONTEXT, "eglCreateContext GLES3");
        require(eglMakeCurrent(session.display, session.surface, session.surface, session.context) == EGL_TRUE,
                "eglMakeCurrent");
        std::cout << "GL_VENDOR: " << text(glGetString(GL_VENDOR)) << '\n'
                  << "GL_RENDERER: " << text(glGetString(GL_RENDERER)) << '\n'
                  << "GL_VERSION: " << text(glGetString(GL_VERSION)) << '\n'
                  << "GLSL: " << text(glGetString(GL_SHADING_LANGUAGE_VERSION)) << '\n';
        const std::string_view renderer{text(glGetString(GL_RENDERER))};
        require(renderer.find("llvmpipe") != std::string_view::npos, "este banco exige llvmpipe software");
        require(glGetError() == GL_NO_ERROR, "error GL antes de los controles");
        pixel_control();
        std::cout << "VERDE [G-01]: contexto GLES3 y pixel RGBA8 correctos\n";
        // Invalid enum has defined API behavior; it is not undefined C++ behavior.
        glEnable(0xFFFFFFFFU);
        require(glGetError() == GL_INVALID_ENUM, "enum invalido no rechazo con GL_INVALID_ENUM");
        require(glGetError() == GL_NO_ERROR, "cola de error no limpia");
        pixel_control();
        std::cout << "VERDE [G-02]: enum rechazado y contexto recuperado\n";
        {
            Shader bad{GL_FRAGMENT_SHADER};
            require(!bad.compile("#version 300 es\nesto no es GLSL valido\n"), "shader invalido aceptado");
            std::array<char, 1024> log{};
            GLsizei used = 0;
            glGetShaderInfoLog(bad.id, static_cast<GLsizei>(log.size()), &used, log.data());
            require(used > 0, "shader rechazado sin log");
            std::cout << "SHADER-ERROR-ESPERADO: " << log.data();
        }
        std::cout << "VERDE [G-03]: GLSL invalido rechazado con diagnostico\n";
        {
            Shader good{GL_FRAGMENT_SHADER};
            require(good.compile("#version 300 es\nprecision highp float;\nout vec4 color;\n"
                                 "void main(){color=vec4(0.0,1.0,0.0,1.0);}\n"), "shader valido rechazado");
        }
        pixel_control();
        std::cout << "VERDE [G-04]: GLSL valido compila despues del rechazo; contexto sigue utilizable\n";
        texture_controls(session, config);
        require(eglMakeCurrent(session.display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT) == EGL_TRUE,
                "desvincular contexto");
        require(eglDestroyContext(session.display, session.context) == EGL_TRUE, "destruir contexto");
        session.context = EGL_NO_CONTEXT;
        require(eglDestroySurface(session.display, session.surface) == EGL_TRUE, "destruir superficie");
        session.surface = EGL_NO_SURFACE;
        require(eglTerminate(session.display) == EGL_TRUE, "terminar EGL");
        session.display = EGL_NO_DISPLAY;
        std::cout << "VERDE [G-05]: recursos EGL liberados\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "ROJO: " << error.what() << '\n';
        return 1;
    }
}

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

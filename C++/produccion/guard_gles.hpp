// SPDX-License-Identifier: GPL-2.0-only
#pragma once
#include "limites.hpp"
#include <EGL/egl.h>
#include <GLES3/gl3.h>
#include <cstdint>
#include <exception>
#include <expected>
#include <limits>
#include <span>
#include <utility>

namespace ultra::gles {
enum class Error { no_context, dirty_state, layout, driver, wrong_context };
// CPU resource ownership only. The EGL context must outlive every Texture.
class Texture {
    GLuint id_ = 0;
    EGLContext owner_ = EGL_NO_CONTEXT;
    Texture(GLuint id, EGLContext owner) : id_(id), owner_(owner) {}
public:
    Texture(const Texture&) = delete;
    Texture& operator=(const Texture&) = delete;
    Texture(Texture&& other) noexcept : id_(std::exchange(other.id_, 0)), owner_(other.owner_) {}
    Texture& operator=(Texture&&) = delete;
    ~Texture() {
        // Deleting this name in another context could delete an unrelated texture.
        if (id_ != 0) {
            if (eglGetCurrentContext() != owner_) std::terminate();
            glDeleteTextures(1, &id_);
        }
    }
    [[nodiscard]] GLuint name() const { return id_; }
    [[nodiscard]] std::expected<void, Error> close() {
        if (id_ == 0) return {};
        if (eglGetCurrentContext() != owner_) return std::unexpected(Error::wrong_context);
        glDeleteTextures(1, &id_);
        id_ = 0;
        return {};
    }
    [[nodiscard]] static std::expected<Texture, Error>
    rgba8(std::int64_t width, std::int64_t height, std::span<const std::uint8_t> pixels,
          std::size_t byte_budget) {
        const auto owner = eglGetCurrentContext();
        if (owner == EGL_NO_CONTEXT) return std::unexpected(Error::no_context);
        // This API owns error observation at its boundary; pre-existing error is rejected.
        if (glGetError() != GL_NO_ERROR) return std::unexpected(Error::dirty_state);
        GLint max_dimension = 0;
        glGetIntegerv(GL_MAX_TEXTURE_SIZE, &max_dimension);
        const auto layout = cpu::rgba8(width, height, pixels.size(), byte_budget, max_dimension);
        if (!layout || width > std::numeric_limits<GLsizei>::max() ||
            height > std::numeric_limits<GLsizei>::max()) return std::unexpected(Error::layout);
        // Ambient PBO/row state must not reinterpret a CPU pointer or read past the span.
        struct State {
            GLint texture = 0, pbo = 0, alignment = 0, row = 0, skip_rows = 0, skip_pixels = 0;
            State() {
                glGetIntegerv(GL_TEXTURE_BINDING_2D, &texture);
                glGetIntegerv(GL_PIXEL_UNPACK_BUFFER_BINDING, &pbo);
                glGetIntegerv(GL_UNPACK_ALIGNMENT, &alignment);
                glGetIntegerv(GL_UNPACK_ROW_LENGTH, &row);
                glGetIntegerv(GL_UNPACK_SKIP_ROWS, &skip_rows);
                glGetIntegerv(GL_UNPACK_SKIP_PIXELS, &skip_pixels);
            }
            ~State() {
                glBindTexture(GL_TEXTURE_2D, static_cast<GLuint>(texture));
                glBindBuffer(GL_PIXEL_UNPACK_BUFFER, static_cast<GLuint>(pbo));
                glPixelStorei(GL_UNPACK_ALIGNMENT, alignment);
                glPixelStorei(GL_UNPACK_ROW_LENGTH, row);
                glPixelStorei(GL_UNPACK_SKIP_ROWS, skip_rows);
                glPixelStorei(GL_UNPACK_SKIP_PIXELS, skip_pixels);
            }
        } previous;
        glBindBuffer(GL_PIXEL_UNPACK_BUFFER, 0);
        glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
        glPixelStorei(GL_UNPACK_ROW_LENGTH, 0);
        glPixelStorei(GL_UNPACK_SKIP_ROWS, 0);
        glPixelStorei(GL_UNPACK_SKIP_PIXELS, 0);
        GLuint id = 0;
        glGenTextures(1, &id);
        if (id == 0) return std::unexpected(Error::driver);
        Texture candidate{id, owner};
        glBindTexture(GL_TEXTURE_2D, id);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
        glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
        glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA8, static_cast<GLsizei>(width),
                     static_cast<GLsizei>(height), 0, GL_RGBA, GL_UNSIGNED_BYTE, pixels.data());
        if (glGetError() != GL_NO_ERROR) return std::unexpected(Error::driver);
        return candidate;
    }
};
}

#pragma once
// stb_image 的 RAII 包装。
//
// stbi_load / stbir_resize 都是裸 malloc，漏一次 free 就泄一整张图。这里统一成
// std::unique_ptr，异常、提前 return、goto 都不用再记着释放。
//
// 删除器用 stbi_image_free：它内部就是 free()，stbir_resize 用 malloc 分配，
// 两边可以共用同一个删除器。

#include <memory>

// dllmain.h 在包含 stb_image.h 之前 define 了 STB_IMAGE_IMPLEMENTATION。
// 这里再包含一次时必须先把它取消，否则实现部分会被编译两遍（C2084）。
#ifdef STB_IMAGE_IMPLEMENTATION
#define STBI_IMAGE_RESTORE_IMPLEMENTATION
#undef STB_IMAGE_IMPLEMENTATION
#endif
#include "../common/stb_image.h"
#ifdef STBI_IMAGE_RESTORE_IMPLEMENTATION
#define STB_IMAGE_IMPLEMENTATION
#undef STBI_IMAGE_RESTORE_IMPLEMENTATION
#endif

/// stb 图像的释放器。
struct StbiImageDeleter
{
    void operator()(unsigned char* data) const noexcept
    {
        if (data != nullptr)
        {
            stbi_image_free(data);
        }
    }
};

/// 一张 stb 图像的独占所有权；空指针表示加载失败。
using StbiImage = std::unique_ptr<unsigned char, StbiImageDeleter>;

/// 对应 `stbi_load`，失败时返回空指针（不会泄漏）。
inline StbiImage stbiLoad(const char* path, int* width, int* height, int* channels, int desired)
{
    return StbiImage{ stbi_load(path, width, height, channels, desired) };
}

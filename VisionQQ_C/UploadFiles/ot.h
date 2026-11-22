#pragma once

// VisionQQ_C 里四个 DLL 的导出声明（UploadFiles.exe 用）。
//
// 这些 DLL 的 .lib 在 VisionQQ_C\x64\Release 下，工程里已经把那个目录加进了
// 库搜索路径；运行时要保证 4 个 DLL 和 exe 在一起（工程里有 post-build 拷贝）。
#include <Windows.h>
#include <cstdio>

struct Point
{
    unsigned x;
    unsigned y;
};
struct Color
{
    unsigned char r, g, b, a;
};
 Color DARK_MODE_MOST_COLOR = { 34, 34, 34, 255 };
// ------------------------------------------------------------------
// 函数指针类型（调用约定必须和原 DLL 一致：这里是默认 __cdecl）
// ------------------------------------------------------------------
typedef int(__cdecl* PFN_fullScreenshot)();
typedef int(__cdecl* PFN_screenshot)(int left, int top, int width, int height);
//extern "C" __declspec(dllexport)
//bool press(WORD vkKey);
//extern "C" __declspec(dllexport)
//WORD getVkKey(const char* key);
typedef int(__cdecl* PFN_press)(WORD vkKey);
typedef int(__cdecl* PFN_getVkKey)(const char* key);

typedef int(__cdecl* PFN_matchTemplatesSelectedScaleBegin)(
    const char* imagePath, const char* templatePath,
    int tolerance, int count, float scale);
typedef Point(__cdecl* PFN_matchTemplateNext)(int subscript);
typedef void(__cdecl* PFN_matchTemplateEnd)();

typedef bool(__cdecl* PFN_DPIAwarenessPrologue)();
typedef bool(__cdecl* PFN_Lclick)(unsigned x, unsigned y);

typedef int(__cdecl* PFN_uploadSelectedFiles)(const wchar_t** filenames, size_t count);
typedef void(__cdecl* PFN_escape)();


typedef Color(__cdecl* PFN_getMostColor)(const char* imagePath, float resize);
typedef bool(__cdecl* PFN_inverted)(const char* imagePath);

//Color getMostColor(const char* imagePath, float resize);
//bool inverted(const char* imagePath)

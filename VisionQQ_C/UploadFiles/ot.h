#pragma once

// VisionQQ_C 里四个 DLL 的导出声明（UploadFiles.exe 用）。
//
// 这些 DLL 的 .lib 在 VisionQQ_C\x64\Release 下，工程里已经把那个目录加进了
// 库搜索路径；运行时要保证 4 个 DLL 和 exe 在一起（工程里有 post-build 拷贝）。

#include <Windows.h>

// --- ScreenCapture.dll ---------------------------------------------------
// fullScreenshot(): 截整个屏幕写到当前目录的 screenshot.png，返回 0 表示成功。
extern "C" __declspec(dllimport) int fullScreenshot();
extern "C" __declspec(dllimport) int screenshot(int left, int top, int width, int height);
#pragma comment(lib, "ScreenCapture.lib")

// --- Vision.dll ----------------------------------------------------------
// matchTemplatesSelectedScaleBegin 在 imagePath 里找 templatePath（模板按 scale 缩放），
// 返回命中的个数（负数表示加载失败）；结果用 matchTemplateNext(i) 逐个取，
// 取完必须 matchTemplateEnd() 释放。
typedef struct Point
{
	unsigned x;
	unsigned y;
} Point;

extern "C" __declspec(dllimport) int matchTemplatesSelectedScaleBegin(
	const char* imagePath,
	const char* templatePath,
	int tolerance,
	int count,
	float scale);
extern "C" __declspec(dllimport) Point matchTemplateNext(int subscript);
extern "C" __declspec(dllimport) void matchTemplateEnd();
#pragma comment(lib, "Vision.lib")

// --- InputEvent.dll ------------------------------------------------------
// DPIAwarenessPrologue(): 把进程设成 DPI 感知，之后坐标/截图都是物理像素。
extern "C" __declspec(dllimport) bool DPIAwarenessPrologue();
// Lclick(): 把光标移到 (x, y) 左键单击一次，坐标是物理屏幕像素。
extern "C" __declspec(dllimport) bool Lclick(unsigned x, unsigned y);
#pragma comment(lib, "InputEvent.lib")

// --- uploadFile.dll ------------------------------------------------------
// uploadSelectedFiles(): 把路径填进标题含"请选择"的上传对话框并点"打开"。
extern "C" __declspec(dllimport) int uploadSelectedFiles(const wchar_t** filenames, size_t count);
#pragma comment(lib, "uploadFile.lib")

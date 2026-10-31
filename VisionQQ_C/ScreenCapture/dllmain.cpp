// dllmain.cpp : 定义 DLL 应用程序的入口点。
#include "pch.h"
#include "dllmain.h"
#include "inicpp.hpp"
#include<iostream>
#include <set>
#include <vector>
#include <cstring>


//  _   _ _____ _____ ___        ____   ___  __  __ 
// | | | |_   _|  ___( _ )   _  | __ ) / _ \|  \/  |
// | | | | | | | |_  / _ \ _| |_|  _ \| | | | |\/| |
// | |_| | | | |  _|| (_) |_   _| |_) | |_| | |  | |
//  \___/  |_| |_|   \___/  |_| |____/ \___/|_|  |_|
//
// 请使用UTF8+BOM                   


using namespace inicpp;

BOOL APIENTRY DllMain(HMODULE hModule,
	DWORD  ul_reason_for_call,
	LPVOID lpReserved
)
{
	switch (ul_reason_for_call)
	{
	case DLL_PROCESS_ATTACH:
	case DLL_THREAD_ATTACH:
	case DLL_THREAD_DETACH:
	case DLL_PROCESS_DETACH:
		break;
	}
	return TRUE;
}

int getScreenW()
{
	int screenW = ::GetSystemMetrics(SM_CXSCREEN);
	return screenW;

}
bool gethdc()
{
	HWND hwd = ::GetDesktopWindow();
	HDC hdc = ::GetDC(hwd);
	if (!hdc) {
		return false;

	}
	::ReleaseDC(hwd, hdc); // 必须释放 DC
	return true;

}
int getWidth()
{
	HWND hwd = GetDesktopWindow();

	HDC hdc = GetDC(hwd);
	int width = 0;
	if (hdc)
	{
		width = ::GetDeviceCaps(hdc, DESKTOPHORZRES);
		::ReleaseDC(hwd, hdc); // 必须释放 DC，否则每次截图泄漏一个 GDI 句柄
	}
	return width;

}

// 把整个屏幕抓成一张 32 位 BMP，直接放进内存。
//
// 以前是 CaptureAnImage 先写 `captureqwsx.bmp`、再 stbi_load 读回来、最后删掉：
// 每截一次图都要落盘一个整屏 BMP（2560x1440 约 14MB）再读一遍，慢、费磁盘，
// 而且中途失败会留下临时文件。现在同样的 GDI 流程，只是把三段 WriteFile
// 换成往 vector 里 memcpy，像素完全一致。
//
// 返回 0 表示成功；`bmp` 里是完整的 BMP 文件内容（文件头 + 信息头 + 像素）。
static int CaptureToMemory(HWND hWnd, const float& scale, std::vector<unsigned char>& bmp)
{
	// 声明全部提到最前面：下面有 goto done，C++ 不允许跳过带初始化的局部变量。
	HDC hdcScreen = GetDC(NULL);
	HDC hdcWindow = GetDC(hWnd);
	HDC hdcMemDC = NULL;
	HBITMAP hbmScreen = NULL;
	HBITMAP hbmOld = NULL;
	HANDLE hDIB = NULL;
	char* lpbitmap = NULL;
	RECT rcClient{};
	BITMAP bmpScreen{};
	BITMAPINFOHEADER bi{};
	BITMAPFILEHEADER bmfHeader{};
	DWORD dwBmpSize = 0;
	int width = 0;
	int height = 0;
	int result = 1;

	// Create a compatible DC, which is used in a BitBlt from the window DC.
	hdcMemDC = CreateCompatibleDC(hdcWindow);
	if (!hdcMemDC)
	{
		MessageBox(hWnd, L"CreateCompatibleDC has failed", L"Failed", MB_OK);
		goto done;
	}

	// Get the client area for size calculation.
	GetClientRect(hWnd, &rcClient);

	// This is the best stretch mode.
	SetStretchBltMode(hdcWindow, HALFTONE);

	// The source DC is the entire screen, and the destination DC is the current window (HWND).
	if (!StretchBlt(hdcWindow,
		0, 0,
		rcClient.right, rcClient.bottom,
		hdcScreen,
		0, 0,
		GetSystemMetrics(SM_CXSCREEN),
		GetSystemMetrics(SM_CYSCREEN),
		SRCCOPY))
	{
		MessageBox(hWnd, L"StretchBlt has failed", L"Failed", MB_OK);
		goto done;
	}

	// Screen Scale
	rcClient.left = static_cast<LONG>(rcClient.left * scale);
	rcClient.top = static_cast<LONG>(rcClient.top * scale);
	rcClient.right = static_cast<LONG>(rcClient.right * scale);
	rcClient.bottom = static_cast<LONG>(rcClient.bottom * scale);

	width = static_cast<int>(rcClient.right - rcClient.left);
	height = static_cast<int>(rcClient.bottom - rcClient.top);
	if (width <= 0 || height <= 0)
	{
		goto done;
	}

	// Create a compatible bitmap from the Window DC.
	hbmScreen = CreateCompatibleBitmap(hdcWindow, width, height);
	if (!hbmScreen)
	{
		MessageBox(hWnd, L"CreateCompatibleBitmap Failed", L"Failed", MB_OK);
		goto done;
	}

	// Select the compatible bitmap into the compatible memory DC.
	// 保存旧位图，释放前恢复，确保新建位图能被 DeleteObject 正常删除
	hbmOld = (HBITMAP)SelectObject(hdcMemDC, hbmScreen);

	// Bit block transfer into our compatible memory DC.
	if (!BitBlt(hdcMemDC, 0, 0, width, height, hdcWindow, 0, 0, SRCCOPY))
	{
		MessageBox(hWnd, L"BitBlt has failed", L"Failed", MB_OK);
		goto done;
	}

	// Get the BITMAP from the HBITMAP.
	GetObject(hbmScreen, sizeof(BITMAP), &bmpScreen);

	bi.biSize = sizeof(BITMAPINFOHEADER);
	bi.biWidth = bmpScreen.bmWidth;
	bi.biHeight = bmpScreen.bmHeight;
	bi.biPlanes = 1;
	bi.biBitCount = 32;
	bi.biCompression = BI_RGB;

	dwBmpSize = ((bmpScreen.bmWidth * bi.biBitCount + 31) / 32) * 4 * bmpScreen.bmHeight;

	hDIB = GlobalAlloc(GHND, dwBmpSize);
	lpbitmap = (char*)GlobalLock(hDIB);
	if (!lpbitmap)
	{
		goto done;
	}

	// Gets the "bits" from the bitmap, and copies them into a buffer
	// that's pointed to by lpbitmap.
	GetDIBits(hdcWindow, hbmScreen, 0,
		(UINT)bmpScreen.bmHeight,
		lpbitmap,
		(BITMAPINFO*)&bi, DIB_RGB_COLORS);

	// 组装 BMP 文件内容：和以前 WriteFile 的那三段（文件头 / 信息头 / 像素）逐字节一致。
	bmfHeader.bfType = 0x4D42; // BM.
	bmfHeader.bfOffBits = (DWORD)sizeof(BITMAPFILEHEADER) + (DWORD)sizeof(BITMAPINFOHEADER);
	bmfHeader.bfSize = dwBmpSize + sizeof(BITMAPFILEHEADER) + sizeof(BITMAPINFOHEADER);

	bmp.resize(bmfHeader.bfSize);
	std::memcpy(bmp.data(), &bmfHeader, sizeof(bmfHeader));
	std::memcpy(bmp.data() + sizeof(bmfHeader), &bi, sizeof(bi));
	std::memcpy(bmp.data() + bmfHeader.bfOffBits, lpbitmap, dwBmpSize);
	result = 0;

	// Clean up.（所有资源统一在 done 释放，正常与出错路径一致）
done:
	if (lpbitmap)
	{
		GlobalUnlock(hDIB);
	}
	if (hDIB)
	{
		GlobalFree(hDIB);
	}
	if (hbmScreen)
	{
		// 先恢复旧位图，确保新建位图能被 DeleteObject 删除
		if (hdcMemDC && hbmOld)
		{
			SelectObject(hdcMemDC, hbmOld);
		}
		DeleteObject(hbmScreen);
	}
	if (hdcMemDC)
	{
		// 兼容 DC 必须用 DeleteDC 释放（DeleteObject 对 DC 无效，会泄漏 GDI 句柄）
		DeleteDC(hdcMemDC);
	}
	if (hdcWindow)
	{
		ReleaseDC(hWnd, hdcWindow);
	}
	if (hdcScreen)
	{
		ReleaseDC(NULL, hdcScreen);
	}

	return result;
}
static bool fileExists(const wchar_t* path)
{
	DWORD attributes = GetFileAttributes(path);
	return (attributes != INVALID_FILE_ATTRIBUTES && !(attributes & FILE_ATTRIBUTE_DIRECTORY));
}
extern "C" __declspec(dllexport)
int fullScreenshot()
{
	float scale = 1.0f;
	if (gethdc())
	{
		scale = float(getWidth()) / getScreenW();
	}
	// 缩放由 screenshot 内部自己算，这里只负责"截全屏 + 确认 png 出来了"
	screenshot(0, 0, getWidth(), getScreenW());
	if (not fileExists(L"screenshot.png"))
	{
		return 1;
	}
	return 0;
}
std::optional<unsigned char*> cropImage(
	const unsigned char* src_data,
	int src_w, int src_h, int channels,
	int crop_x, int crop_y, int crop_w, int crop_h)
{
	// 1. 边界检查与修正
	if (crop_x < 0) crop_x = 0;
	if (crop_y < 0) crop_y = 0;

	// 如果起点就在图片外，直接失败
	if (crop_x >= src_w || crop_y >= src_h) {
		return std::nullopt; 
	}

	// 修正裁剪尺寸以防止越界
	if (crop_x + crop_w > src_w) crop_w = src_w - crop_x;
	if (crop_y + crop_h > src_h) crop_h = src_h - crop_y;

	if (crop_w <= 0 || crop_h <= 0) {
		return std::nullopt;
	}

	// 2. 分配内存 (调用者负责 delete[])
	// 注意：使用 new[] 而不是 new，因为我们要分配数组
	unsigned char* destData = new(std::nothrow) unsigned char[crop_w * crop_h * channels];
	if (!destData) {
		return std::nullopt; // 内存分配失败
	}

	const int src_row_stride = src_w * channels;
	const int dst_row_stride = crop_w * channels;
	const int copy_bytes = dst_row_stride;

	// 3. 逐行拷贝
	for (int y = 0; y < crop_h; ++y) {
		const unsigned char* src_row_ptr = src_data + ((crop_y + y) * src_row_stride) + (crop_x * channels);
		unsigned char* dst_row_ptr = destData + (y * dst_row_stride);
		std::memcpy(dst_row_ptr, src_row_ptr, copy_bytes);
	}

	return destData;
}

// 只抓需要的那一块，直接写成 png。
//
// 老路是"整屏抓图 -> 整屏 GetDIBits -> 拼 BMP -> stb 解码 -> 裁剪 -> 写 png"，
// 中间那一大坨（2560x1440 约 14MB 的拷贝 + 解码）在小块截图时纯属浪费。
// 这里只对目标矩形做 BitBlt/GetDIBits，再按 stb 读 32 位 BMP 的规则做
// BGRA -> RGBA 和行翻转（BMP 是自下而上），所以输出和走老路逐像素一致。
//
// 返回 0 成功、3 区域越界、4 写文件失败、-1 表示 GDI 这步没走通（交给老路）。
static int ScreenshotRegionToPng(int left, int top, int width, int height, const char* out_file)
{
	HWND hWnd = GetDesktopWindow();
	HDC hdcScreen = GetDC(NULL);
	HDC hdcWindow = GetDC(hWnd);
	HDC hdcMemDC = CreateCompatibleDC(hdcWindow);
	HBITMAP hbmScreen = NULL;
	HBITMAP hbmOld = NULL;
	HANDLE hDIB = NULL;
	char* lpbitmap = NULL;

	auto cleanup = [&]()
	{
		if (lpbitmap)
		{
			GlobalUnlock(hDIB);
		}
		if (hDIB)
		{
			GlobalFree(hDIB);
		}
		if (hbmScreen)
		{
			if (hdcMemDC && hbmOld)
			{
				SelectObject(hdcMemDC, hbmOld);
			}
			DeleteObject(hbmScreen);
		}
		if (hdcMemDC)
		{
			DeleteDC(hdcMemDC);
		}
		if (hdcWindow)
		{
			ReleaseDC(hWnd, hdcWindow);
		}
		if (hdcScreen)
		{
			ReleaseDC(NULL, hdcScreen);
		}
	};

	// 边界处理和 cropImage + 外层那两段完全一样
	RECT rcClient{};
	GetClientRect(hWnd, &rcClient);
	const int img_w = static_cast<int>(rcClient.right - rcClient.left);
	const int img_h = static_cast<int>(rcClient.bottom - rcClient.top);

	if (left < 0) left = 0;
	if (top < 0) top = 0;
	if (left >= img_w || top >= img_h)
	{
		cleanup();
		return 3;
	}
	if (left + width > img_w) width = img_w - left;
	if (top + height > img_h) height = img_h - top;
	if (width <= 0 || height <= 0)
	{
		cleanup();
		return 3;
	}

	if (!hdcMemDC)
	{
		cleanup();
		return -1;
	}

	hbmScreen = CreateCompatibleBitmap(hdcWindow, width, height);
	if (!hbmScreen)
	{
		cleanup();
		return -1;
	}
	hbmOld = (HBITMAP)SelectObject(hdcMemDC, hbmScreen);

	// 从**屏幕 DC** 直接拷这一块：老路是先把整屏 StretchBlt 到桌面窗口 DC 再 BitBlt，
	// 两步在 1:1 时等价，但从桌面窗口 DC 取会漏掉分层窗口（比如置顶的半透明浮窗），
	// 直接从屏幕 DC 取才是"眼睛看到的"。
	if (!BitBlt(hdcMemDC, 0, 0, width, height, hdcScreen, left, top, SRCCOPY))
	{
		cleanup();
		return -1;
	}

	BITMAPINFOHEADER bi{};
	bi.biSize = sizeof(BITMAPINFOHEADER);
	bi.biWidth = width;
	bi.biHeight = height; // 正数 = 自下而上，和以前 GetDIBits 拿到的行序一致
	bi.biPlanes = 1;
	bi.biBitCount = 32;
	bi.biCompression = BI_RGB;

	const DWORD dwBmpSize = static_cast<DWORD>(width) * static_cast<DWORD>(height) * 4;
	hDIB = GlobalAlloc(GHND, dwBmpSize);
	lpbitmap = static_cast<char*>(GlobalLock(hDIB));
	if (!lpbitmap)
	{
		cleanup();
		return -1;
	}
	if (GetDIBits(hdcWindow, hbmScreen, 0, static_cast<UINT>(height), lpbitmap,
		reinterpret_cast<BITMAPINFO*>(&bi), DIB_RGB_COLORS) == 0)
	{
		cleanup();
		return -1;
	}

	// stb 读 32 位 BI_RGB 的 BMP 时是 4 通道，而且"整张图 alpha 全 0"会把 alpha 补成 255。
	std::vector<unsigned char> rgba(static_cast<size_t>(width) * height * 4);
	const unsigned char* src = reinterpret_cast<const unsigned char*>(lpbitmap);
	bool any_alpha = false;
	for (int y = 0; y < height; ++y)
	{
		const unsigned char* src_row = src + static_cast<size_t>(height - 1 - y) * width * 4;
		unsigned char* dst_row = rgba.data() + static_cast<size_t>(y) * width * 4;
		for (int x = 0; x < width; ++x)
		{
			const unsigned char b = src_row[x * 4 + 0];
			const unsigned char g = src_row[x * 4 + 1];
			const unsigned char r = src_row[x * 4 + 2];
			const unsigned char a = src_row[x * 4 + 3];
			if (a != 0)
			{
				any_alpha = true;
			}
			dst_row[x * 4 + 0] = r;
			dst_row[x * 4 + 1] = g;
			dst_row[x * 4 + 2] = b;
			dst_row[x * 4 + 3] = a;
		}
	}
	if (!any_alpha)
	{
		for (size_t i = 3; i < rgba.size(); i += 4)
		{
			rgba[i] = 255;
		}
	}

	const int ok = stbi_write_png(out_file, width, height, 4, rgba.data(), 0) != 0;
	cleanup();
	return ok ? 0 : 4;
}

extern "C" __declspec(dllexport)
int screenshot(int left, int top, int width, int height)
{
	// 1. 计算缩放比例
	float scale = 1.0f;
	if (gethdc()) {
		// 防止除以零
		int screenW = getScreenW();
		if (screenW > 0) {
			scale = static_cast<float>(getWidth()) / screenW;
		}
	}

	const char* out_file = "screenshot.png";

	// 2. 缩放为 1（DPI 感知的调用方都是这种情况）时走快路：
	//    只抓目标矩形，直接写 png，不经过 BMP。
	if (scale > 0.999f && scale < 1.001f)
	{
		const int fast = ScreenshotRegionToPng(left, top, width, height, out_file);
		if (fast >= 0)
		{
			return fast;
		}
	}

	// 3. 缩放不为 1（进程没设 DPI 感知时）：整屏抓图、缩放、再从整屏里裁剪。
	//    整屏 BMP 只放在内存里，不再写 captureqwsx.bmp 再读回来。
	std::vector<unsigned char> bmp;
	if (CaptureToMemory(GetDesktopWindow(), scale, bmp) != 0)
	{
		return 1; // 截图失败
	}

	// 4. 从内存解码（等价于以前的 stbi_load("captureqwsx.bmp", ...)）
	int img_w, img_h, channels;
	unsigned char* image = stbi_load_from_memory(
		bmp.data(), static_cast<int>(bmp.size()), &img_w, &img_h, &channels, 0);

	if (!image) {
		return 2; // 加载图片失败
	}

	// 5. 执行裁剪
	// 传入正确的原图尺寸 (img_w, img_h) 和 用户想要的裁剪参数 (left, top, width, height)
	auto cropped_opt = cropImage(image, img_w, img_h, channels, left, top, width, height);

	// 释放源图片内存 (stb_image 分配的必须用 stbi_image_free)
	stbi_image_free(image);

	if (!cropped_opt.has_value()) {
		// 裁剪失败 (可能是区域越界)
		return 3;
	}

	unsigned char* cropped_ptr = cropped_opt.value();

	int final_w = width;
	int final_h = height;

	// 简单的边界钳制逻辑复现，确保写入尺寸正确
	if (left < 0) left = 0;
	if (top < 0) top = 0;
	if (left + final_w > img_w) final_w = img_w - left;
	if (top + final_h > img_h) final_h = img_h - top;

	if (final_w <= 0 || final_h <= 0) {
		delete[] cropped_ptr; // 清理内存
		return 3;
	}

	bool write_success = stbi_write_png(out_file, final_w, final_h, channels, cropped_ptr,0);

	// 6. 清理内存 (对应 cropImage 中的 new[])
	// 无论保存是否成功，都必须删除，防止内存泄漏
	delete[] cropped_ptr;

	if (!write_success) {
		return 4; // 保存失败
	}

	return 0; // 成功
}

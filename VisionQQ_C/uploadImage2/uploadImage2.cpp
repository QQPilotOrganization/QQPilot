//命令行程序，直接复制图片到剪贴板
#define _CRT_SECURE_NO_WARNINGS
#include <Windows.h>
#include <GdiPlus.h>
#include <iostream>
#include <strsafe.h>
#include<cstdlib>
#include <memory>
#include<algorithm>
#include<vector>
#include<ctime>
#include<chrono>
#include<random>
//#include<pathcch.h>
#pragma comment(lib, "GdiPlus")

bool copyimage(const wchar_t* filename);
std::wstring anypath;
static int select()
{


	const wchar_t* imageExts[] = { L".gif",L".png",L".jpg",L".jpeg",L".webp",L".avif",L".bmp" };
	//列出所有文件
	WIN32_FIND_DATA findFileData;

	HANDLE hFind = INVALID_HANDLE_VALUE;
	WCHAR pathwithastar[MAX_PATH];
	WCHAR path[MAX_PATH];

	GetCurrentDirectory(MAX_PATH, pathwithastar);
	std::wcscat(pathwithastar, L"\\Images\\*");
	//std::wcscat(pathwithastar, L"\\Images\\");
	std::wcsncpy(path, pathwithastar, std::wcslen(pathwithastar) - 1);

	std::wcout << pathwithastar << std::endl;
	std::wcout << path << std::endl;
	;
	std::vector<std::wstring> images;

	hFind = FindFirstFile(pathwithastar, &findFileData);
	if (hFind == INVALID_HANDLE_VALUE)
	{
		std::wcerr << "错误:" << GetLastError() << std::endl;
		return -2;

	}
	do {
		if (std::wcscmp(findFileData.cFileName, L"..") == 0)
		{
			continue;
		}
		if (std::wcscmp(findFileData.cFileName, L".") == 0)
		{
			continue;
		}
		//图片文件
		for (int i = 0; i < 6; ++i)
		{
			const wchar_t* ext = imageExts[i];
			size_t length = std::wcslen(ext);
			//std::wcout << ext << length;
			//std::cout << std::endl;
			size_t fileNameLength = std::wcslen(findFileData.cFileName);
			if (std::wcscmp(findFileData.cFileName + fileNameLength + 1 - length, ext) == 0)
			{


				goto Picture;
			}

		}
	Picture:
		//std::wcout << findFileData.cFileName << std::endl;
		images.emplace_back(std::wstring(findFileData.cFileName));
	} while (FindNextFile(hFind, &findFileData));


	unsigned seed = std::chrono::system_clock::now().time_since_epoch().count();
	std::shuffle(images.begin(), images.end(), std::default_random_engine(seed));
	if (images.size() == 0)
	{
		std::wcerr << TEXT("没有找到图片") << std::endl;
		return -1;
	}
	anypath = std::wstring(path, std::wcslen(pathwithastar) - 1) + images.at(0);
}

std::unique_ptr<wchar_t[]> CharToWchar(const char* str) {
	if (!str) return nullptr;

	// Step 1: 获取所需缓冲区大小
	int wideLen = MultiByteToWideChar(
		CP_ACP,           // 代码页：ANSI
		0,                // 标志位
		str,              // 输入字符串
		-1,               // -1 表示自动计算到 '\0'
		nullptr,          // 首次调用
		0                 // 返回所需大小
	);

	if (wideLen <= 0) {
		throw std::runtime_error("MultiByteToWideChar failed");
	}

	// Step 2: 分配内存（auto 释放）
	auto wideStr = std::make_unique<wchar_t[]>(wideLen);

	// Step 3: 执行转换
	int result = MultiByteToWideChar(
		CP_ACP,
		0,
		str,
		-1,
		wideStr.get(),
		wideLen
	);

	if (result == 0) {
		throw std::runtime_error("MultiByteToWideChar conversion failed");
	}

	return wideStr; // 自动管理内存
}
int main(int argc,char** argv)
{
	if (argc == 2)
	{
		//selectedImage = argv[1];
		auto result=CharToWchar(argv[1]);
		anypath = result.get();
	}
	else if (argc==1)
	{
		select();
	}
	else
	{
		std::cout << "Invalid argument:\t" << argv[0] << " <selectedImage>\n";
		return -1;
	}



	Gdiplus::GdiplusStartupInput gdiplusStartupInput;
	ULONG_PTR gdiplusToken;
	Gdiplus::GdiplusStartup(&gdiplusToken, &gdiplusStartupInput, NULL);
	std::wcout << anypath << std::endl;
	bool result = copyimage(anypath.c_str());

	std::cout << result;
	Gdiplus::GdiplusShutdown(gdiplusToken);
}
bool copyimage(const wchar_t* filename)
{
	bool result = false;
	Gdiplus::Bitmap* gdibmp = Gdiplus::Bitmap::FromFile(filename);
	if (gdibmp)
	{
		HBITMAP hbitmap;
		gdibmp->GetHBITMAP(0, &hbitmap);
		if (OpenClipboard(NULL))
		{
			EmptyClipboard();
			DIBSECTION ds;
			if (GetObject(hbitmap, sizeof(DIBSECTION), &ds))
			{
				HDC hdc = GetDC(HWND_DESKTOP);
				//create compatible bitmap (get DDB from DIB)
				HBITMAP hbitmap_ddb = CreateDIBitmap(hdc, &ds.dsBmih, CBM_INIT,
					ds.dsBm.bmBits, (BITMAPINFO*)&ds.dsBmih, DIB_RGB_COLORS);
				ReleaseDC(HWND_DESKTOP, hdc);
				SetClipboardData(CF_BITMAP, hbitmap_ddb);
				DeleteObject(hbitmap_ddb);
				result = true;
			}
			CloseClipboard();
		}

		//cleanup:
		DeleteObject(hbitmap);
		delete gdibmp;
	}
	return result;
}
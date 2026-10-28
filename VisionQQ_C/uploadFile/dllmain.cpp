// dllmain.cpp : 定义 DLL 应用程序的入口点。

//用UTF8+BOM	就能正常使用了是何意味


//何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味何意味

//  _   _ _____ _____ ___        ____   ___  __  __ 
// | | | |_   _|  ___( _ )   _  | __ ) / _ \|  \/  |
// | | | | | | | |_  / _ \ _| |_|  _ \| | | | |\/| |
// | |_| | | | |  _|| (_) |_   _| |_) | |_| | |  | |
//  \___/  |_| |_|   \___/  |_| |____/ \___/|_|  |_|
//
// 请使用UTF8+BOM                                               



#include "pch.h"

#ifdef _DEBUG
#include<iostream>
#endif
BOOL APIENTRY DllMain( HMODULE hModule,
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


std::wstring anypath;
const wchar_t* findStr = L"请选择";
static BOOL CALLBACK uploadFile(HWND hwnd, LPARAM lparam);
static BOOL escapeUploadFileDialog(HWND hwnd, LPARAM lparam);
bool fileExists(const wchar_t* fileName);
static bool stringInString(const std::wstring& text, const std::wstring& search)
{
	return text.find(search) != std::wstring::npos;
}




#include <string>
#include <vector>
#include <sstream>

static std::wstring wjoin(const std::wstring& delimiter, const std::vector<std::wstring>& parts) {
	if (parts.empty()) {
		return L"";
	}

	std::wstringstream ss;
	for (size_t i = 0; i < parts.size(); ++i) {
		if (i > 0) {
			ss << delimiter;
		}
		ss << parts[i];
	}
	return ss.str();
}


extern "C" int __declspec(dllexport) uploadSelectedFiles(const wchar_t** filenames,size_t count)
{	
	std::vector<std::wstring> existFiles;
	existFiles.reserve(count);
	for (size_t i = 0; i < count; i++)
	{
		const wchar_t* filename = filenames[i];
		if (filename == nullptr || filename[0] == L'\0')
		{
			continue;
		}
		if (not fileExists(filename))
		{
			continue;
		}

		// 路径里可能有空格，统一用引号包起来；已经带引号的就不要再加一层。
		std::wstring path(filename);
		if (path.front() != L'\"')
		{
			path.insert(path.begin(), L'\"');
		}
		if (path.back() != L'\"')
		{
			path.push_back(L'\"');
		}
		existFiles.push_back(path);
	}


	anypath = wjoin(L" ", existFiles);
#ifdef  _DEBUG
	std::wcout<<L"Anypath:" << anypath << std::endl;
#endif //  _DEBUG
	EnumWindows(uploadFile, NULL);
	return 0;
}
bool fileExists(const wchar_t* fileName)
{
	wchar_t workingDirectory[MAX_PATH]{};
	GetCurrentDirectory(MAX_PATH, workingDirectory);
	if (std::wcslen(workingDirectory) > MAX_PATH - 1)
	{
		return false;
	}
	wchar_t combined[MAX_PATH * 2] = {0};
	PathCchCombineEx(combined, MAX_PATH * 2, workingDirectory, fileName, PATHCCH_NONE);

	DWORD attributes = GetFileAttributes(combined);
	if (attributes != INVALID_FILE_ATTRIBUTES)
	{
		return !(attributes & FILE_ATTRIBUTE_DIRECTORY);
	}
	return false;
}

extern "C" int __declspec(dllexport) upload()
{


	const wchar_t* imageExts[] = { L".gif",L".png",L".jpg",L".jpeg",L".webp",L".avif",L".bmp" };
	//列出所有文件
	WIN32_FIND_DATA findFileData;
	
	HANDLE hFind = INVALID_HANDLE_VALUE;
	WCHAR pathwithastar[MAX_PATH];
	WCHAR path[MAX_PATH];

	GetCurrentDirectory(MAX_PATH, pathwithastar);
	std::wcscat(pathwithastar, L"\\Images\\*");
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
	
	std::wprintf(L"->%s", images.at(0).c_str());
	// 上传文件
	EnumWindows(uploadFile, NULL);
	return 0;
}
extern "C" int __declspec(dllexport) uploadSelectedImage(wchar_t* src)
{
	anypath = src;
	std::wprintf(L"->%s", src);

	EnumWindows(uploadFile, NULL);
	return 0;
}


extern "C" void __declspec(dllexport) escape()
{
	EnumWindows(escapeUploadFileDialog, NULL);
}
static std::wstring getWindowName(const HWND& hwnd)
{
	if (not IsWindowVisible(hwnd))
	{
		return L"";
	}
	int length = GetWindowTextLength(hwnd);
	if (not length) return L"";
	std::unique_ptr<wchar_t[]> windowName = std::make_unique<wchar_t[]>(static_cast<size_t>(length) + 1);
	int success = GetWindowText(hwnd, windowName.get(), MAX_PATH);
	if (not success)
	{
		return L"";
	}
	//return true;
	//std::wcout << windowName.get() << std::endl;
	const std::wstring windowName2 = std::wstring(windowName.get(), length);
	return windowName2;
}

static BOOL CALLBACK uploadFile(HWND hwnd, LPARAM lparam)
{
	//wchar_t windowName[MAX_PATH]{};

	auto windowName2 = getWindowName(hwnd);
	if (windowName2 == L"") return true;
	if (stringInString(windowName2, findStr))
	{
#ifdef _DEBUG
		std::wcout << L"Found Window:" << hwnd<<L"\n"<<"name:"<<windowName2<<"\n";
#endif // _DEBUG

		HWND comboboxEx32 = FindWindowEx(hwnd, NULL, L"ComboBoxEx32", NULL);
		HWND combobox = FindWindowEx(comboboxEx32, NULL, L"ComboBox", NULL);
		HWND edit = FindWindowEx(combobox, NULL, L"Edit", NULL);
		SendMessage(edit, WM_SETTEXT, NULL, (LPARAM)((anypath).c_str()));
		HWND button = FindWindowEx(hwnd, NULL, L"Button", NULL);
		SendMessage(hwnd, WM_COMMAND, 1, (LPARAM)button);
	}
	return true;
}
static BOOL CALLBACK escapeUploadFileDialog(HWND hwnd, LPARAM lparam)
{
	auto windowName2 = getWindowName(hwnd);
	if (windowName2 == L"") return true;
	if (stringInString(windowName2, findStr))
	{
		SendMessage(hwnd, WM_CLOSE, 0, 0);
		return false;
	}
	return true;
}

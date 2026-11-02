#include "pch.h"
#include "findTencentFiles.h"
#include <optional>   
#include <string>

std::optional<std::wstring> findTencentFiles()
{       
    //TencentFiles Ò»°ãÔÚÎÄµµÎÄ¼þ¼ÐÖÐ
	wchar_t* path;
	HRESULT hr = SHGetKnownFolderPath(FOLDERID_Documents, 0, NULL, &path);
    if (not SUCCEEDED(hr))
    { 
         return std::nullopt;
    }
    wchar_t tencentPath[MAX_PATH];
    PathCchCombineEx(tencentPath, MAX_PATH, path, L"Tencent Files", PATHCCH_NONE);
    if (PathFileExists(tencentPath))
    {
        return (tencentPath);
    }
    return std::nullopt;
}
static bool _findEveryFiles(const std::wstring& searchPath, std::vector<std::wstring>& files)
{
    wchar_t path[MAX_PATH];
    //std::wcscpy(path, searchPath.c_str());
    PathCchCombineEx(path, MAX_PATH, searchPath.c_str(), L"\\*", PATHCCH_NONE);
    HANDLE hFind = INVALID_HANDLE_VALUE;
    WIN32_FIND_DATA ffd;
    hFind=FindFirstFile(path, &ffd);
    if (hFind == INVALID_HANDLE_VALUE)
    {
        return false;
    }
    do
    {
        if (ffd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY)
        {
            if (wcscmp(ffd.cFileName, L".") != 0 && wcscmp(ffd.cFileName, L"..") != 0)
            {
                wchar_t subPath[MAX_PATH];
                PathCchCombineEx(subPath, MAX_PATH, searchPath.c_str(), ffd.cFileName, PATHCCH_NONE);
                _findEveryFiles(subPath, files);
            }
        }
        else
        {
            files.push_back(ffd.cFileName);
        }
    }   while (FindNextFile(hFind, &ffd) != 0); 
    FindClose(hFind);
    return true;
}

void findEveryFiles2(const std::filesystem::path& dir, std::vector<std::filesystem::path>& files) 
{
    for (const auto& entry : std::filesystem::recursive_directory_iterator(dir)) 
    {
        if (entry.is_regular_file()) 
        {
            files.push_back(entry.path());
        }
    }
}

void listFolders(const std::filesystem::path& path, std::vector<std::filesystem::path>& folders)
{

    for (const auto& entry : std::filesystem::directory_iterator(path))
    {
        if (entry.is_directory())
        {
            folders.emplace_back(entry.path());

        }
    }

}

bool folderValid(const std::filesystem::path& path)
{
    std::filesystem::path tencentPath = path;
    tencentPath /= L"nt_data";
    tencentPath /= L"Emoji";
    return std::filesystem::exists(tencentPath);
}
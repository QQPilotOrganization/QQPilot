// ScaleToINI.cpp : 此文件包含 "main" 函数。程序执行将在此处开始并结束。
//

#include <iostream>
#include "..\common\inicpp.hpp"
#include <windows.h>
#include <string>
#include<sstream>

#include <iomanip>
using namespace inicpp;

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
    int width = ::GetDeviceCaps(hdc, DESKTOPHORZRES);
    return width;

}
int main()
{
    if (not gethdc())
    {
        return -1;
    }
    IniManager ini("config.ini");
    float scale = float(getWidth()) / getScreenW();
    std::ostringstream ost;
    ost << std::fixed << std::setprecision(2) << scale;

    //ini["general"]["scale"] = std::to_string(scale);
    ini.set("general", "scale", ost.str());
}
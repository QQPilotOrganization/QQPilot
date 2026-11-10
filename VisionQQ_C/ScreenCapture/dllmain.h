#pragma once
#define	 STBI_ONLY_BMP
#define STB_IMAGE_IMPLEMENTATION
#include "..\common\stb_image.h"
#define STB_IMAGE_WRITE_IMPLEMENTATION
#include "..\common\stb_image_write.h"
#include <vector>
#include<optional>

extern "C" __declspec(dllexport)
int screenshot(int left, int top, int width, int height);

extern "C" __declspec(dllexport) int fullScreenshot();
/*
对屏幕的(left,top,width,height)位置截图
	  返回值为1，截图失败
*/

//extern "C" __declspec(dllexport)
//Point containsRedDot(RECT rect);
///*
//指定RECT是否有红点
//返回位置，若返回(0,0)则无
//*/
//
//extern "C" __declspec(dllexport)
//Point containsBlue();
///*
//指定RECT是否有蓝色登录按钮
//返回位置，若返回(0,0)则无
//*/
//
//extern "C" __declspec(dllexport)
//Point point(unsigned x, unsigned y); 
//
//
//
//extern "C" __declspec(dllexport)
//RECT rect(unsigned left, unsigned top, unsigned right, unsigned bottom);

#pragma once
#define _CRT_SECURE_NO_WARNINGS
#include"pch.h"
#include "../common/stb_image.h"
#include "../common/stb_image_write.h"
#include "../common/stb_image_resize2.h"
#include"pixel.h"
#include"stbImage.h"
#include<cassert>

/// 深色模式下 QQ 的主背景色（#222222）。
extern "C" __declspec(dllexport)
const Color DARK_MODE_MOST_COLOR = { 34, 34, 34, 255 };

/// 取整张图里出现次数最多的颜色；`resize` 是采样比例（0~1，超出范围按 1 处理）。
extern "C" __declspec(dllexport)
Color getMostColor(const char* imagePath, float resize);

/// 把图片反色（RGB，保留 alpha）后原地写回；成功返回 true。
extern "C" __declspec(dllexport)
bool inverted(const char* imagePath);

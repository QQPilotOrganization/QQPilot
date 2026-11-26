#pragma once
#define _CRT_SECURE_NO_WARNINGS

//#define	 STBI_ONLY_BMP
// 只保留项目实际用到的格式：截图是 PNG，模板是 PNG/JPEG（有些 .png 其实是 JPEG）。
// 其余解码器（GIF/BMP/TGA/PSD/HDR/PIC/PNM）全裁掉，能少十几 KB。
#define STBI_ONLY_PNG
#define STBI_ONLY_JPEG
#define STB_IMAGE_IMPLEMENTATION
#include "../common/stb_image.h"
#define STB_IMAGE_WRITE_IMPLEMENTATION
#include "../common/stb_image_write.h"
#define STB_IMAGE_RESIZE_IMPLEMENTATION
#include "../common/stb_image_resize2.h"
#include"Point.hpp"
#define EXPORT extern "C" __declspec(dllexport)

typedef struct Point
{
	unsigned x;
	unsigned y;
} Point;
EXPORT RECT rect(unsigned x, unsigned y, unsigned w, unsigned h)
{
	return { (long)x, (long)y, (long)w, (long)h };
}
typedef  unsigned char Image;
struct PackedImage
{
	Image* src;
	int width, height, channels;
};
EXPORT int matchTemplatesScalesBegin(const char* imagePath, const char* templatePath, int tolerance, int count, const float* scales, int numScales);
EXPORT Point matchTemplateNext(int subscript);
EXPORT void matchTemplateEnd();
EXPORT float matchTemplateHitScale();

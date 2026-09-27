#pragma once
#define _CRT_SECURE_NO_WARNINGS

//#define	 STBI_ONLY_BMP
#define STB_IMAGE_IMPLEMENTATION
#include "stb_image.h"
//#define STB_IMAGE_WRITE_IMPLEMENTATION
//#include "stb_image_write.h"
#define STB_IMAGE_RESIZE_IMPLEMENTATION
#include "stb_image_resize2.h"
#include"Point.hpp"
#define EXPORT extern "C" __declspec(dllexport)

typedef struct Point
{
	unsigned x;
	unsigned y;
} Point;
EXPORT Point point(unsigned x, unsigned y)
{
	return { x,y };
}
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
EXPORT int matchTemplatesBegin(const char* imagePath, const char* templatePath, int tolerance, int count);
EXPORT Point matchTemplateNext(int subscript);
EXPORT void matchTemplateEnd();
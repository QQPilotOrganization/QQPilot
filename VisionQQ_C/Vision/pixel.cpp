#include"pch.h"
#include "pixel.h"

std::array<unsigned char, 4> getPixel(const unsigned char* image, const int x, const int y, const int width, int channels)
{
	const unsigned char* pixelOffset = image + (x + y * width) * channels;
	std::array<unsigned char, 4> result{ 0 };
	result[0] = pixelOffset[0];
	if (channels >= 2) {
		result[1] = pixelOffset[1];
	}

	if (channels >= 3) {
		result[2] = pixelOffset[2];
	}

	if (channels >= 4) {
		result[3] = pixelOffset[3];
	}

	return result;

}

void putPixel(unsigned char* image, const int x, const int y, const int width, int channels,const std::array<unsigned char, 4>& pixel)
{
	 unsigned char* pixelOffset = image + (x + y * width) * channels;
	 pixelOffset[0] = pixel.at(0);
	if (channels >= 2) {
		pixelOffset[1] = pixel.at(1);
	}

	if (channels >= 3) {
		pixelOffset[2] = pixel.at(2);
	}

	if (channels >= 4) {
		pixelOffset[3] = pixel.at(3);
	}


}
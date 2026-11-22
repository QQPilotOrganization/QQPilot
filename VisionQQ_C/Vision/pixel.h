#pragma once
#include "pch.h"
std::array<unsigned char, 4> getPixel(const unsigned char* image, const int x, const int y, const int width, int channels);
void putPixel(unsigned char* image, const int x, const int y, const int width, int channels, const std::array<unsigned char, 4>& pixel);
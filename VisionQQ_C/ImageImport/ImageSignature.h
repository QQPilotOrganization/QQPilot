#pragma once
#include <iostream>
#include <fstream>
#include <vector>
#include <string>
#include <cstdint>
#include <array>
#include<expected>
#include<stdexcept>

// 定义常见图片格式的特征码 (Magic Numbers)
struct ImgSignature {
    std::vector<uint8_t> magic;
    const wchar_t* format;
    const wchar_t* ext;
};

const std::vector<ImgSignature> SIGNATURES = {
    { { 0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A }, L"PNG", L"png" },
    { { 0xFF, 0xD8, 0xFF }, L"JPEG", L"jpg" },
    { { 0x47, 0x49, 0x46, 0x38, 0x37, 0x61 }, L"GIF87a", L"gif" },
    { { 0x47, 0x49, 0x46, 0x38, 0x39, 0x61 }, L"GIF89a", L"gif" },
    { { 0x52, 0x49, 0x46, 0x46 }, L"RIFF", L"webp" },
    { { 0x42, 0x4D }, L"BMP", L"bmp" },
    { { 0x00, 0x00, 0x01, 0x00 }, L"ICO", L"ico" },
    { { 0x49, 0x49, 0x2A, 0x00 }, L"TIFF (LE)", L"tiff" },
    { { 0x4D, 0x4D, 0x00, 0x2A }, L"TIFF (BE)", L"tiff" },
    { { 0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20 }, L"JPEG 2000", L"jp2" },
    { { 0x38, 0x42, 0x50, 0x53 }, L"PSD", L"psd" }
};
std::expected<std::wstring, std::exception> getRealExtension(const wchar_t* src);
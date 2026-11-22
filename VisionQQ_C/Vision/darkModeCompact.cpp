#include"pch.h"
#include"darkModeCompact.h"
#include<map>

extern "C" __declspec(dllexport)
Color getMostColor(const char* imagePath, float resize)
{
	if (resize > 1.0f or resize <= 0.0f)
	{
		resize = 1.0f;
	}
	int img_w = 0, img_h = 0, img_channels = 0;
	StbiImage imageData = stbiLoad(imagePath, &img_w, &img_h, &img_channels, 4);
	if (!imageData) {
		return { 0,0,0,0 };
	}
	img_channels = 4;

	int new_width = static_cast<int> (std::floorf(img_w * resize));
	int new_height = static_cast<int> (std::floorf(img_h * resize));
	if (new_width <= 0 || new_height <= 0) {
		return { 0,0,0,0 };
	}

	// 缩放；stbir_resize_uint8_linear 在目标指针为 nullptr 时自动 malloc，
	// 交给 StbiImage 释放（stbi_image_free 就是 free）
	StbiImage resized{
		stbir_resize_uint8_linear(
			imageData.get(),
			img_w, img_h,
			0,                  // src_stride (0 = 紧凑)
			nullptr,            // dst_buffer (nullptr = 自动分配)
			new_width,
			new_height,
			0,                  // dst_stride (0 = 紧凑)
			STBIR_RGBA          // 格式：必须与后续匹配逻辑一致
		)
	};
	if (!resized) {
		return { 0,0,0,0 };
	}

	std::map<std::array<unsigned char, 4>, unsigned> colors;
	for (int y = 0; y < new_height; ++y)
	{
		for (int x = 0; x < new_width; ++x)
		{
			const auto color = getPixel(resized.get(), x, y, new_width, img_channels);
			++colors[color];
		}
	}
	if (colors.empty())
	{
		return { 0,0,0,0 };
	}

	// 出现次数最多的颜色
	const auto maxIt = *std::max_element(colors.begin(), colors.end(),
		[](const std::pair<std::array<unsigned char, 4>, unsigned>& kv1,
		   const std::pair<std::array<unsigned char, 4>, unsigned>& kv2)
		{
			return kv1.second < kv2.second;
		}
	);
	return{
		maxIt.first.at(0),
		maxIt.first.at(1),
		maxIt.first.at(2),
		maxIt.first.at(3),
	};
}

extern "C" __declspec(dllexport)
bool inverted(const char* imagePath)
{
	int img_w = 0, img_h = 0, img_channels = 0;
	StbiImage imageData = stbiLoad(imagePath, &img_w, &img_h, &img_channels, 4);
	if (!imageData) {
		return false;
	}

	// 只反 RGB，alpha 不动
	for (int y = 0; y < img_h; ++y)
	{
		for (int x = 0; x < img_w; ++x)
		{
			auto pixel = getPixel(imageData.get(), x, y, img_w, img_channels);
			pixel[0] = 255 - pixel[0];
			if (img_channels >= 2) {
				pixel[1] = 255 - pixel[1];
			}
			if (img_channels >= 3) {
				pixel[2] = 255 - pixel[2];
			}

			putPixel(imageData.get(), x, y, img_w, img_channels, pixel);
		}
	}

	return stbi_write_png(imagePath, img_w, img_h, img_channels, imageData.get(), 0) != 0;
}

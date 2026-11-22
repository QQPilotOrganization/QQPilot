// dllmain.cpp : 定义 DLL 应用程序的入口点。
#include "pch.h"
#include "dllmain.h"
#include "pixel.h"
#include "stbImage.h"
#ifdef _WIN32
#define EXPORT extern "C" __declspec(dllexport)
#else
#define EXPORT extern "C" __attribute__((visibility("default")))
#endif
BOOL APIENTRY DllMain(HMODULE hModule,
	DWORD  ul_reason_for_call,
	LPVOID lpReserved
)
{
	switch (ul_reason_for_call)
	{
	case DLL_PROCESS_ATTACH:
	case DLL_THREAD_ATTACH:
	case DLL_THREAD_DETACH:
	case DLL_PROCESS_DETACH:
		break;
	}
	return TRUE;
}

static int MatchTemplateByFactors(const int numFactors, const float factors[], PackedImage& originalTplObj, int template_w, int template_h, int img_w, int img_h, int template_channels, PackedImage& imgObj, int tolerance, int count);

/// 缩放模板；失败（尺寸不合法/分配失败）时返回空指针。
static StbiImage resize(const PackedImage& input, float factor) {
    int new_width = static_cast<int>(input.width * factor);
    int new_height = static_cast<int>(input.height * factor);

    // 1. 边界检查
    if (new_width <= 0 || new_height <= 0) {
        return {};
    }

    // 2. 执行缩放
    // stbir_resize_uint8_linear 在目标指针为 nullptr 时会自动 malloc，
    // 这里用 StbiImage 接管（stbi_image_free 就是 free）
    return StbiImage{ stbir_resize_uint8_linear(
        input.src,
        input.width,
        input.height,
        0,                  // src_stride (0 = 紧凑)
        nullptr,            // dst_buffer (nullptr = 自动分配)
        new_width,
        new_height,
        0,                  // dst_stride (0 = 紧凑)
        STBIR_RGBA          // 格式：必须与后续匹配逻辑一致
    ) };
}



Point contains(RECT rect, const Color& color, const char* path);
EXPORT
Point containsRedDot(RECT rect,const char* path)
{        
	const Color RED_DOT = { 247,76,48,255 };
	return contains(rect, RED_DOT,path);
}
EXPORT
Point containsBlue( const char* path)
{
	const Color RED_DOT = { 0,153,255,255 };
	return contains({0,0,0,0}, RED_DOT, path);
}
Point contains(RECT rect, const Color& color,const char* path)
{
    //std::printf("%s\n", path);
	int img_w, img_h, channels;
    //std::printf("%ld,%ld,%ld,%ld\n", rect.left, rect.right, rect.top, rect.bottom);
    StbiImage image = stbiLoad(path, &img_w, &img_h, &channels, 0);
    if (!image)
    {
        return { 0,0 };
    }
    if (rect.bottom == 0 and rect.right == 0 and rect.left == 0 and rect.top == 0)
    {
        rect = { 0,0,img_w,img_h };
    }
    //std::printf("%ld,%ld,%ld,%ld\n", rect.left, rect.right, rect.top, rect.bottom);

    if (rect.bottom < 0 or rect.right < 0 or rect.left < 0 or rect.top < 0)
    {
        std::cout << 1;
        goto end;
    }
    if (rect.bottom > img_h or rect.right > img_w or rect.left > img_w or rect.top > img_h)
    {
        std::cout << 2;
        goto end;
    }



	for (int x = rect.left; x < rect.right; ++x)
	{
		for (int y = rect.top; y < rect.bottom; ++y)
		{
			std::array<Image, 4> result = getPixel(image.get(), x, y, img_w, channels);

			if (result[0] == color.r and result[1] == color.g and result[2] == color.b)
			{
				return{ static_cast<unsigned>(x),static_cast<unsigned>(y) };
			}
		}

	}
    end:
	return{ 0,0 };
}
static std::vector<Point> revelants;

/**
 * 核心匹配函数
 * @param toleranceThreshold 用于早期退出的阈值提示 (总分 = 平均分 * 像素数)
 */
static double matchScore(const PackedImage& image, const PackedImage& templ, Point position, double toleranceThreshold) {
    // 边界检查 (position 是 unsigned，所以只需检查上界)
    // 注意：这里需要把 position 转回 int 进行减法运算，防止 unsigned 下溢
    int px = static_cast<int>(position.x);
    int py = static_cast<int>(position.y);

    if (px < 0 || py < 0 ||
        px + templ.width > image.width ||
        py + templ.height > image.height) {
        return (std::numeric_limits<double>::max)();
    }

    const int channels = image.channels;
    const int templWidth = templ.width;
    const int templHeight = templ.height;

    const int imageStride = image.width * channels;
    const int templStride = templWidth * channels;

    // 计算起始指针
    const unsigned char* imgRowPtr = image.src + (py * image.width + px) * channels;
    const unsigned char* tplRowPtr = templ.src;

    double totalDiff = 0.0;
    // 早期退出阈值：如果总分超过这个值，平均分肯定超过 toleranceThreshold
    const double maxAllowedTotalDiff = toleranceThreshold * (templWidth * templHeight * channels);

    for (int ty = 0; ty < templHeight; ++ty) {
        const unsigned char* imgPixel = imgRowPtr;
        const unsigned char* tplPixel = tplRowPtr;

        for (int tx = 0; tx < templWidth; ++tx) {
            for (int c = 0; c < channels; ++c) {
                int diff = static_cast<int>(*imgPixel) - static_cast<int>(*tplPixel);
                totalDiff += std::abs(diff);

                // 【性能优化】早期退出
                if (totalDiff > maxAllowedTotalDiff) {
                    return (std::numeric_limits<double>::max)();
                }

                ++imgPixel;
                ++tplPixel;
            }
        }
        imgRowPtr += imageStride;
        tplRowPtr += templStride;
    }

    const double normalizationFactor = static_cast<double>(templWidth * templHeight * channels);
    return totalDiff / normalizationFactor;
}

// ==========================================
// DLL 导出接口
// ==========================================
void _matchTemplate(const PackedImage& imgObj, const PackedImage& tplObj, int tolerance,int maxCount, std::vector<Point>& dest = revelants);
static bool isPointDuplicate(const std::vector<Point>& existingPoints, const Point& newPt, int minDistance) {
    for (const auto& pt : existingPoints) {
        int dx = static_cast<int>(pt.x) - static_cast<int>(newPt.x);
        int dy = static_cast<int>(pt.y) - static_cast<int>(newPt.y);
        // 如果横纵坐标差值都小于阈值，视为重复
        if (std::abs(dx) < minDistance && std::abs(dy) < minDistance) {
            return true;
        }
    }
    return false;
}

EXPORT
int matchTemplatesSelectedScaleBegin(const char* imagePath, const char* templatePath, int tolerance, int count,float scale)
{
    revelants.clear(); // 清空全局结果列表

    const float factors[] = { scale };
    const int numFactors = sizeof(factors) / sizeof(factors[0]);

    // 1. 加载大图 (只加载一次)
    int img_w, img_h, img_channels;
    StbiImage image_data = stbiLoad(imagePath, &img_w, &img_h, &img_channels, 4);
    if (!image_data) {
        return -2;
    }
    img_channels = 4;

    // 2. 加载原始模板 (只加载一次)
    int template_w, template_h, template_channels;
    StbiImage template_data = stbiLoad(templatePath, &template_w, &template_h, &template_channels, 4);
    if (!template_data) {
        return -3;
    }
    template_channels = 4;

    PackedImage imgObj = { image_data.get(), img_w, img_h, img_channels };
    PackedImage originalTplObj = { template_data.get(), template_w, template_h, template_channels };

    // 3. 多尺度循环（两块内存由上面的 StbiImage 负责释放）
    return MatchTemplateByFactors(numFactors, factors, originalTplObj, template_w, template_h, img_w, img_h, template_channels, imgObj, tolerance, count);
}

EXPORT
int matchTemplatesMultiScaleBegin(const char* imagePath, const char* templatePath, int tolerance,int count)
{
    revelants.clear(); // 清空全局结果列表

    const float factors[] = { 1.0f,1.25f,1.5f,1.75f,2.0f,2.25f,2.5f,2.75f,3.0f, 1.1f, 1.2f, 1.3f, 1.4f, 1.6f, 1.7f, 1.8f, 1.9f };
    const int numFactors = sizeof(factors) / sizeof(factors[0]);

    // 1. 加载大图 (只加载一次)
    int img_w, img_h, img_channels;
    StbiImage image_data = stbiLoad(imagePath, &img_w, &img_h, &img_channels, 4);
    if (!image_data) {
        return -2;
    }
    img_channels = 4;

    // 2. 加载原始模板 (只加载一次)
    int template_w, template_h, template_channels;
    StbiImage template_data = stbiLoad(templatePath, &template_w, &template_h, &template_channels, 4);
    if (!template_data) {
        return -3;
    }
    template_channels = 4;

    PackedImage imgObj = { image_data.get(), img_w, img_h, img_channels };
    PackedImage originalTplObj = { template_data.get(), template_w, template_h, template_channels };

    // 3. 多尺度循环（两块内存由上面的 StbiImage 负责释放）
    return MatchTemplateByFactors(numFactors, factors, originalTplObj, template_w, template_h, img_w, img_h, template_channels, imgObj, tolerance, count);
}



static int MatchTemplateByFactors(const int numFactors, const float factors[], PackedImage& originalTplObj, int template_w, int template_h, int img_w, int img_h, int template_channels, PackedImage& imgObj, int tolerance, int count)
{
    for (int i = 0; i < numFactors; ++i) {
        float factor = factors[i];
        std::cout << "f " << factor << "\r";

        PackedImage currentTplObj;
        StbiImage resizedData;   // 只有缩放时才会分配，离开作用域自动释放
        int currentTplW = 0;
        int currentTplH = 0;

        if (factor == 1.0f) {
            // 1.0 倍直接使用原始数据
            currentTplObj = originalTplObj;
            currentTplW = template_w;
            currentTplH = template_h;
        }
        else {
            // 其他倍数进行 Resize
            resizedData = resize(originalTplObj, factor);
            if (!resizedData) {
                continue; // 缩放失败，跳过
            }

            currentTplW = static_cast<int>(template_w * factor);
            currentTplH = static_cast<int>(template_h * factor);

            // 如果缩放后的模板比大图还大，跳过 (避免无效计算)
            if (currentTplW > img_w || currentTplH > img_h) {
                continue; // resizedData 自动释放
            }

            currentTplObj = { resizedData.get(), currentTplW, currentTplH, template_channels };
        }

        // --- 核心逻辑：匹配 -> 去重 -> 合并 ---

        // 1. 调用现有的 _matchTemplate (它会向 dest 追加新结果)
        std::vector<Point> newMatches;

        _matchTemplate(imgObj, currentTplObj, tolerance, count, newMatches);
        if (newMatches.size() >= count)
        {
            std::copy(newMatches.begin(), newMatches.end(), std::back_inserter(revelants));
            // 原来这里是 goto end 手动 free，现在交给 StbiImage
            return static_cast<int>(revelants.size());
        }

        // 2. 动态计算去重阈值 (基于当前模板大小，例如模板宽度的 20%)
        int dedupThreshold = static_cast<int>(currentTplW * 0.2f);
        if (dedupThreshold < 5) dedupThreshold = 5; // 最小阈值保护

        // 3. 将新结果去重后加入全局列表
        std::copy_if(newMatches.begin(), newMatches.end(), std::back_inserter(revelants), [dedupThreshold](const Point& pt) {return !isPointDuplicate(revelants, pt, dedupThreshold); });
    }

    // 4. 返回最终去重后的总数
    return static_cast<int>(revelants.size());
}

void _matchTemplate(const PackedImage& imgObj, const PackedImage& tplObj, int tolerance, int maxCount,std::vector<Point>& dest)
{
    //std::cout<< "id:" << std::this_thread::get_id() << "\n";
    int max_y = imgObj.height - tplObj.height;
    int max_x = imgObj.width - tplObj.width;

    // 边界检查：如果模板比大图大，直接返回
    if (max_y < 0 || max_x < 0) return;

    for (int y = 0; y <= max_y; ++y) {
        //std::cout << "row " << y << std::endl;
        for (int x = 0; x <= max_x; ++x) {
            Point pos;
            pos.x = static_cast<unsigned>(x);
            pos.y = static_cast<unsigned>(y);

            double score = matchScore(imgObj, tplObj, pos, static_cast<double>(tolerance));

            if (score < tolerance) {
                dest.push_back(pos);

                if (static_cast<int>(dest.size()) >= maxCount) {
                    return;
                }
            }
        }
    }
}

EXPORT
int matchTemplatesBegin(const char* imagePath, const char* templatePath, int tolerance,int count)
{
    revelants.clear();

    // 1. 加载大图
    int img_w, img_h, img_channels;
    StbiImage image_data = stbiLoad(imagePath, &img_w, &img_h, &img_channels, 4);
    img_channels = 4;
    if (!image_data) {
        return -2; // 大图加载失败
    }

    // 2. 加载模板图
    int template_w, template_h, template_channels;
    StbiImage template_data = stbiLoad(templatePath, &template_w, &template_h, &template_channels, 4);
    template_channels = 4;
    if (!template_data) {
        return -3; // 模板加载失败
    }

    // 3. 基础检查
    if (img_w < template_w || img_h < template_h) {
        return -1; // 尺寸不够
    }
    PackedImage imgObj = { image_data.get(), img_w, img_h, img_channels };
    PackedImage tplObj = { template_data.get(), template_w, template_h, template_channels };

    _matchTemplate(imgObj, tplObj, tolerance,count,revelants);

    // 4. 两块图像内存在这里随 StbiImage 析构释放

    return static_cast<int>(revelants.size());
}

EXPORT
Point matchTemplateNext(int subscript)
{
    // 边界检查
    if (subscript < 0 || subscript >= static_cast<int>(revelants.size())) {
        // 返回 {0,0} 作为默认值。
        Point invalid = { 0, 0 };
        return invalid;
    }
    return revelants[subscript];
}

EXPORT
void matchTemplateEnd()
{
    //revelants.clear();

     std::vector<Point>().swap(revelants); // 速度不行了占用总要低一点吧
}
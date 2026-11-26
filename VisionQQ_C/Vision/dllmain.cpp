// dllmain.cpp : 定义 DLL 应用程序的入口点。
#include "pch.h"
#include "dllmain.h"
#include "pixel.h"
#include "stbImage.h"
#include <cstdlib>
#include <cstring>
#if defined(_M_X64)
#include <immintrin.h> // SSE2 / AVX2：模板匹配的内层 SAD
#endif
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
Point containsRedDot(RECT rect, const char* path)
{
	const Color RED_DOT = { 247,76,48,255 };
	return contains(rect, RED_DOT, path);
}
EXPORT
Point containsBlue(const char* path)
{
	const Color RED_DOT = { 0,153,255,255 };
	return contains({ 0,0,0,0 }, RED_DOT, path);
}
Point contains(RECT rect, const Color& color, const char* path)
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
/// 最近一次产生结果的倍数（多尺度匹配用；单尺度时就是传进去的 scale）。
static float g_lastMatchScale = 0.0f;

// ==========================================================================
// 内层 SAD（一行像素的绝对差之和）：
//   AVX2（32 字节/指令）   >   SSE2（16 字节/指令）   >   标量
// 用 CPUID 在运行期选，同一个 DLL 在新旧 CPU 上都能跑；不支持 AVX2 的机器
// 自动退回 SSE2，连 SSE2 都没有（非 x64）才走标量。
// ==========================================================================
enum class SimdBackend { Scalar, Sse2, Avx2 };

static SimdBackend detectSimdBackend() {
#if defined(_M_X64)
	// 测试用：VISION_SIMD=sse2 / scalar 可以强制走回退路径，验证两条路结果一致
	if (const char* forced = std::getenv("VISION_SIMD")) {
		if (std::strcmp(forced, "sse2") == 0) return SimdBackend::Sse2;
		if (std::strcmp(forced, "scalar") == 0) return SimdBackend::Scalar;
	}

	int regs[4] = {};
	__cpuid(regs, 1);
	const bool osxsave = (regs[2] & (1 << 27)) != 0;
	const bool avx = (regs[2] & (1 << 28)) != 0;
	// AVX2 还要操作系统保存 YMM 状态（XCR0 的 bit1/bit2），否则执行 AVX 指令会 #UD
	if (osxsave && avx && (_xgetbv(0) & 0x6) == 0x6) {
		__cpuidex(regs, 7, 0);
		if ((regs[1] & (1 << 5)) != 0) {
			return SimdBackend::Avx2; // AVX2
		}
	}
	return SimdBackend::Sse2; // x64 上 SSE2 是基线，任何 x64 CPU 都有
#else
	return SimdBackend::Scalar;
#endif
}

static const SimdBackend g_simd = detectSimdBackend();

/// 当前实际使用的 SIMD 后端：0=标量 1=SSE2 2=AVX2（诊断用，老 CPU 上会返回 1 或 0）。
EXPORT
int visionSimdBackend() {
	return static_cast<int>(g_simd);
}

static inline long long sadRowScalar(const unsigned char* a, const unsigned char* b, int bytes) {
	long long total = 0;
	for (int i = 0; i < bytes; ++i) {
		total += std::abs(static_cast<int>(a[i]) - static_cast<int>(b[i]));
	}
	return total;
}

#if defined(_M_X64)
static inline long long sadRowSse2(const unsigned char* a, const unsigned char* b, int bytes) {
	__m128i acc = _mm_setzero_si128();
	int i = 0;
	for (; i + 16 <= bytes; i += 16) {
		acc = _mm_add_epi64(acc, _mm_sad_epu8(_mm_loadu_si128(reinterpret_cast<const __m128i*>(a + i)),
			_mm_loadu_si128(reinterpret_cast<const __m128i*>(b + i))));
	}
	long long total = _mm_cvtsi128_si64(acc) + _mm_cvtsi128_si64(_mm_unpackhi_epi64(acc, acc));
	for (; i < bytes; ++i) {
		total += std::abs(static_cast<int>(a[i]) - static_cast<int>(b[i]));
	}
	return total;
}

/// AVX2 版本：32 字节块用 vpsadbw，剩下的先用 vzeroupper 再走 SSE2/标量尾巴。
static inline long long sadRowAvx2(const unsigned char* a, const unsigned char* b, int bytes) {
	__m256i acc = _mm256_setzero_si256();
	int i = 0;
	for (; i + 32 <= bytes; i += 32) {
		acc = _mm256_add_epi64(acc, _mm256_sad_epu8(_mm256_loadu_si256(reinterpret_cast<const __m256i*>(a + i)),
			_mm256_loadu_si256(reinterpret_cast<const __m256i*>(b + i))));
	}
	const __m128i lo = _mm256_castsi256_si128(acc);
	const __m128i hi = _mm256_extracti128_si256(acc, 1);
	const __m128i lanes = _mm_add_epi64(lo, hi);
	long long total = _mm_cvtsi128_si64(lanes) + _mm_cvtsi128_si64(_mm_unpackhi_epi64(lanes, lanes));

	_mm256_zeroupper(); // 后面还要用 SSE，清掉 YMM 高位，避免 AVX/SSE 切换惩罚
	for (; i + 16 <= bytes; i += 16) {
		const __m128i s = _mm_sad_epu8(_mm_loadu_si128(reinterpret_cast<const __m128i*>(a + i)),
			_mm_loadu_si128(reinterpret_cast<const __m128i*>(b + i)));
		total += _mm_cvtsi128_si64(s) + _mm_cvtsi128_si64(_mm_unpackhi_epi64(s, s));
	}
	for (; i < bytes; ++i) {
		total += std::abs(static_cast<int>(a[i]) - static_cast<int>(b[i]));
	}
	return total;
}
#endif

static inline long long sadRow(const unsigned char* a, const unsigned char* b, int bytes) {
#if defined(_M_X64)
	if (g_simd == SimdBackend::Avx2) return sadRowAvx2(a, b, bytes);
	if (g_simd == SimdBackend::Sse2) return sadRowSse2(a, b, bytes);
#endif
	return sadRowScalar(a, b, bytes);
}

/**
 * 核心匹配函数
 * @param toleranceThreshold 用于早期退出的阈值提示 (总分 = 平均分 * 像素数)
 *
 * 逐行累加 SAD，一旦超过阈值就提前返回；累加是整数，结果和逐像素版本完全一致，
 * 只是把内层循环换成了 SSE2。
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
	if (image.src == nullptr || templ.src == nullptr ||
		image.channels <= 0 || templ.width <= 0 || templ.height <= 0) {
		return (std::numeric_limits<double>::max)();
	}

	const int channels = image.channels;
	const int rowBytes = templ.width * channels;
	const int templHeight = templ.height;
	const int imageStride = image.width * channels;

	// 计算起始指针
	const unsigned char* imgRowPtr = image.src + (py * image.width + px) * channels;
	const unsigned char* tplRowPtr = templ.src;

	// 早期退出阈值：如果总分超过这个值，平均分肯定超过 toleranceThreshold
	const double maxAllowedTotalDiff = toleranceThreshold * static_cast<double>(rowBytes * templHeight);

	long long totalDiff = 0;
	for (int ty = 0; ty < templHeight; ++ty) {
		totalDiff += sadRow(imgRowPtr, tplRowPtr, rowBytes);

		// 【性能优化】早期退出（按行检查即可，模板一般只有十几行）
		if (static_cast<double>(totalDiff) > maxAllowedTotalDiff) {
			return (std::numeric_limits<double>::max)();
		}

		imgRowPtr += imageStride;
		tplRowPtr += rowBytes;
	}

	const double normalizationFactor = static_cast<double>(rowBytes * templHeight);
	return static_cast<double>(totalDiff) / normalizationFactor;
}

// ==========================================
// DLL 导出接口
// ==========================================
void _matchTemplate(const PackedImage& imgObj, const PackedImage& tplObj, int tolerance, int maxCount, std::vector<Point>& dest = revelants);
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
int matchTemplatesSelectedScaleBegin(const char* imagePath, const char* templatePath, int tolerance, int count, float scale)
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
int matchTemplatesMultiScaleBegin(const char* imagePath, const char* templatePath, int tolerance, int count)
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

/// 自定义倍数表的多尺度匹配：大图和模板只加载一次，按 scales 的顺序逐个匹配，
/// 凑够 count 个（或全部试完）就返回。用于「在 dScale 附近试 7 个倍数」这类场景，
/// 不必每个倍数都重新解码一遍截图。
EXPORT
int matchTemplatesScalesBegin(const char* imagePath, const char* templatePath, int tolerance, int count, const float* scales, int numScales)
{
	revelants.clear(); // 清空全局结果列表

	if (scales == nullptr || numScales <= 0) {
		return -4; // 倍数表非法
	}

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
	return MatchTemplateByFactors(numScales, scales, originalTplObj, template_w, template_h, img_w, img_h, template_channels, imgObj, tolerance, count);
}



static int MatchTemplateByFactors(const int numFactors, const float factors[], PackedImage& originalTplObj, int template_w, int template_h, int img_w, int img_h, int template_channels, PackedImage& imgObj, int tolerance, int count)
{
	for (int i = 0; i < numFactors; ++i) {
		float factor = factors[i];

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
		if (!newMatches.empty()) {
			g_lastMatchScale = factor;
		}
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

/// 逐行扫描，收集所有 score < tolerance 的位置（最多 maxCount 个），坐标就是 imgObj 的像素坐标。
///
/// 这里刻意不做「缩到 1/4 再扫」：仓库里的模板只有 14x18 ~ 23x28 像素，缩到 1/4 后
/// 只剩 3x4 ~ 5x7，细节全被抹掉 —— 实测 copy.png / file.png 会误报，且返回的是
/// 1/4 坐标系里的点（调用方拿去点击会偏 4 倍）。速度改由 matchScore 里的 SSE2
/// 内层循环提供，结果和旧版逐像素实现完全一致。
static void _matchTemplate(const PackedImage& imgObj, const PackedImage& tplObj, int tolerance, int maxCount, std::vector<Point>& dest)
{
	if (tplObj.width <= 0 || tplObj.height <= 0) return;

	int max_y = imgObj.height - tplObj.height;
	int max_x = imgObj.width - tplObj.width;

	// 边界检查：如果模板比大图还大，直接返回
	if (max_y < 0 || max_x < 0) return;

	const double threshold = static_cast<double>(tolerance);
	for (int y = 0; y <= max_y; ++y) {
		for (int x = 0; x <= max_x; ++x) {
			const Point pos{ static_cast<unsigned>(x), static_cast<unsigned>(y) };

			const double score = matchScore(imgObj, tplObj, pos, threshold);
			if (score < threshold) {
				dest.push_back(pos);

				if (static_cast<int>(dest.size()) >= maxCount) {
					return;
				}
			}
		}
	}
}
EXPORT
int matchTemplatesBegin(const char* imagePath, const char* templatePath, int tolerance, int count)
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

	_matchTemplate(imgObj, tplObj, tolerance, count, revelants);

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

/// 最近一次匹配实际用的倍数。多尺度调用后用它算模板中心，(0 表示还没匹配过)
EXPORT
float matchTemplateHitScale()
{
	return g_lastMatchScale;
}

EXPORT
void matchTemplateEnd()
{
	//revelants.clear();

	std::vector<Point>().swap(revelants); // 速度不行了占用总要低一点吧
}
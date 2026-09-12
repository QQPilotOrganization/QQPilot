#include "pch.h"


#include "mouseEvent.hpp"
#include "windmouse.h"

extern "C" __declspec(dllexport)
bool Mousegoto(unsigned x, unsigned y)
{
	if (x > 32768 or y > 32768)
	{
		return false;
	}
	INPUT input = { 0 };
	input.type = INPUT_MOUSE;
	int cx = GetSystemMetrics(SM_CXSCREEN);
	int cy = GetSystemMetrics(SM_CYSCREEN);
	if (cx <= 0 || cy <= 0) return false;
	input.mi.dx = static_cast<LONG>(x * 65535 / (cx - 1));
	input.mi.dy = static_cast<LONG>(y * 65535 / (cy - 1));
	input.mi.dwFlags = MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE;
	return SendInput(1, &input, sizeof(INPUT)) == 1;
}	  


extern "C" __declspec(dllexport)
bool SmoothMousegoto(unsigned x, unsigned y)
{
	if (x > 32768 or y > 32768)
	{
		return false;
	}
	//std::cout << x << "," << y << "\n";
	POINT startPosition;
	BOOL success=GetCursorPos(&startPosition);
	//std::cout << startPosition.x << "\n";
	//std::cout << startPosition.y << "\n";
	if ( not success)
	{
		startPosition.x = 0;
		startPosition.y = 0;
	}
	WindMouse windMouse(10);
	MouseSettings mouseSettings{ startPosition.x,startPosition.y,x,y };

	mouseSettings.gravity = 16.0f;
	mouseSettings.wind = 1.0f;
	mouseSettings.maxStep = 60;
	mouseSettings.targetArea = 20.0f;
	mouseSettings.minWait = 1;
	mouseSettings.maxWait = 2;

	float distance = std::hypot(std::abs(static_cast<float>(x) - static_cast<float>(startPosition.x)), std::abs(static_cast<float>(y) - static_cast<float>(startPosition.y)));
	float split =50;
	split = std::floorf(split);

	std::cout << split<< std::endl;
	mouseSettings.maxStep = split;
	std::vector<MousePoint> p=windMouse.GeneratePoints(mouseSettings);
	int waitbefore = 0;
	for (auto point : p)
	{
		Mousegoto(point.x, point.y);
		std::this_thread::sleep_for(std::chrono::milliseconds(point.wait-waitbefore));
		waitbefore = point.wait;
		//std::cout <<
		//	"|" << point.x << "," << point.y << "|" << point.wait << std::endl;
	}
	return true;
}



extern "C" __declspec(dllexport)
bool scrollUp(int delta)
{
	INPUT input = { 0 };
	input.type = INPUT_MOUSE;
	input.mi.mouseData = static_cast<DWORD>(delta);
	input.mi.dwFlags = MOUSEEVENTF_WHEEL;
	return SendInput(1, &input, sizeof(INPUT)) == 1;
}

extern "C" __declspec(dllexport)
bool scrollDown(int delta)
{
	return scrollUp(-delta);
}

extern "C" __declspec(dllexport)
bool scrollLeft(int delta)
{
	INPUT input = { 0 };
	input.type = INPUT_MOUSE;
	input.mi.mouseData = static_cast<DWORD>(delta);
	input.mi.dwFlags = MOUSEEVENTF_HWHEEL; // ×¢Òâ£ºHWHEEL
	return SendInput(1, &input, sizeof(INPUT)) == 1;
}


extern "C" __declspec(dllexport)
bool scrollRight(int delta)
{
	return scrollLeft(-delta);
}


extern "C" __declspec(dllexport)
bool dragFromTo(unsigned x1, unsigned y1, unsigned x2, unsigned y2, float durationSeconds)
{
	if (!Mousegoto(x1, y1)) return false;
	if (!LmouseDown()) return false;

	// Æ½»¬ÍÏ×§µ½ÖÕµã
	tweenPosition(x1, y1, x2, y2, durationSeconds);

	// ¿ÉÑ¡£ºÍÏ×§½áÊøºóÔÙ±£³ÖÒ»Ð¡¶ÎÊ±¼ä
	std::this_thread::sleep_for(std::chrono::milliseconds(50));

	return LmouseUp();
}
extern "C" __declspec(dllexport)
bool LmouseDown()
{
	INPUT input = { 0 };
	input.type = INPUT_MOUSE;
	input.mi.dwFlags = MOUSEEVENTF_LEFTDOWN;
	return SendInput(1, &input, sizeof(INPUT)) == 1;
}

extern "C" __declspec(dllexport)
bool LmouseUp()
{
	INPUT input = { 0 };
	input.type = INPUT_MOUSE;
	input.mi.dwFlags = MOUSEEVENTF_LEFTUP;
	return SendInput(1, &input, sizeof(INPUT)) == 1;
}
extern "C" __declspec(dllexport)
bool RmouseUp()
{
	INPUT input = { 0 };
	input.type = INPUT_MOUSE;
	input.mi.dwFlags = MOUSEEVENTF_RIGHTUP;
	return SendInput(1, &input, sizeof(INPUT)) == 1;
}
extern "C" __declspec(dllexport)
bool RmouseDown()
{
	INPUT input = { 0 };
	input.type = INPUT_MOUSE;
	input.mi.dwFlags = MOUSEEVENTF_RIGHTDOWN;
	return SendInput(1, &input, sizeof(INPUT)) == 1;
}
void tweenPosition(unsigned startX, unsigned startY,
	unsigned endX, unsigned endY,
	float durationSeconds)
{
	if (durationSeconds <= 0.0f) {
		Mousegoto(endX, endY);
		return;
	}


	const int totalSteps = static_cast<int>(durationSeconds * 100.0f); // 10ms per step
	const float invSteps = 1.0f / static_cast<float>(totalSteps);

	int screenW = GetSystemMetrics(SM_CXSCREEN);
	int screenH = GetSystemMetrics(SM_CYSCREEN);
	if (screenW <= 1 || screenH <= 1) return;

	for (int i = 0; i <= totalSteps; ++i) {
		float t = min(1.0f, static_cast<float>(i) * invSteps); // t ¡Ê [0,1]

		// ÏßÐÔ²åÖµ
		unsigned x = static_cast<unsigned>(startX + (endX - startX) * t);
		unsigned y = static_cast<unsigned>(startY + (endY - startY) * t);

		// ¹éÒ»»¯µ½ 0~65535
		LONG dx_norm = static_cast<LONG>(x * 65535 / (screenW - 1));
		LONG dy_norm = static_cast<LONG>(y * 65535 / (screenH - 1));

		INPUT input = { 0 };
		input.type = INPUT_MOUSE;
		input.mi.dx = dx_norm;
		input.mi.dy = dy_norm;
		input.mi.dwFlags = MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE;

		SendInput(1, &input, sizeof(INPUT));

		if (i < totalSteps) {
			std::this_thread::sleep_for(std::chrono::milliseconds(10));
		}
	}
}

extern "C" __declspec(dllexport)
bool Lclick(unsigned x, unsigned y)
{
	if (!Mousegoto(x, y)) return false;
	if (!LmouseDown()) return false;
	std::this_thread::sleep_for(std::chrono::milliseconds(10));
	return LmouseUp();
}
extern "C" __declspec(dllexport)
bool Rclick(unsigned x, unsigned y)
{
	if (!Mousegoto(x, y)) return false;
	if (!RmouseDown()) return false;
	std::this_thread::sleep_for(std::chrono::milliseconds(10));
	return RmouseUp();
}
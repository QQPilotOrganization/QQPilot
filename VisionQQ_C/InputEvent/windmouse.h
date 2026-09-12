#pragma once

#include <vector>
#include <cmath>
#include <random>
#include <algorithm>
#include <cstdint>
#include<generator>

struct MouseSettings {
    float startX;
    float startY;
    float endX;
    float endY;
    float gravity= 5.0f;
    float wind= 2.0f;
    float maxStep= 10.0f;
    float targetArea= 20.0f;
    int minWait=1;
    int maxWait=5;
};

// 轨迹点结构：[x, y, wait_time]
struct MousePoint {
    int x;
    int y;
    int wait;
};

class WindMouse {
private:
    float mouseSpeed;
    int randomSeed;
    float randomSpeed;

    std::default_random_engine generator;
    std::uniform_real_distribution<float> rand2;

    // 对应 TS 的 Hypot

public:
    explicit WindMouse(float speed)
        : mouseSpeed(speed),
        generator(std::random_device{}()),
        rand2(0.0, 10.0)
    {
        randomSeed = static_cast<int>(std::floor(rand2(generator)));
        randomSpeed = std::max<float>((static_cast<float>(randomSeed) / 2.0 + mouseSpeed) / 10.0, 0.1);
    }

    std::vector<MousePoint> GeneratePoints(MouseSettings settings);
    std::generator<MousePoint> YieldPoints(MouseSettings settings);
};
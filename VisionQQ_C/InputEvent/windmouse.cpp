#include "pch.h"
#include "windmouse.h"

std::vector<MousePoint> WindMouse::GeneratePoints(MouseSettings settings) {

    if (settings.gravity < 1.0) settings.gravity = 1.0;
    if (settings.maxStep == 0.0) settings.maxStep = 0.01;

    float dist;
    float windX = std::floor(rand2(generator));
    float windY = std::floor(rand2(generator));
    float velocityX = 0.0;
    float velocityY = 0.0;
    float randomDist;
    float veloMag;
    float step;

    float    oldX, oldY;
    float    newX = std::round(settings.startX);
    float    newY = std::round(settings.startY);

    const float waitDiff = static_cast<double>(settings.maxWait - settings.minWait);
    const float sqrt2 = std::sqrt(2.0);
    const float sqrt3 = std::sqrt(3.0);
    const float sqrt5 = std::sqrt(5.0);

    std::vector<MousePoint> points;
    int currentWait = 0;

    dist = std::hypotf(settings.endX - settings.startX, settings.endY - settings.startY);

    while (dist > 1.0) {
        settings.wind = std::min<float>(settings.wind, dist);

        if (dist >= settings.targetArea)
        {
            std::uniform_int_distribution<int> wind_dist(0, static_cast<int>(std::round(settings.wind)) * 2);
            int w = wind_dist(generator);

            windX = windX / sqrt3 + static_cast<double>(w - settings.wind) / sqrt5;
            windY = windY / sqrt3 + static_cast<double>(w - settings.wind) / sqrt5;
        }
        else
        {
            windX /= sqrt2;
            windY /= sqrt2;
            if (settings.maxStep < 3.0) {
                std::uniform_int_distribution<int> step_dist(3, 5);
                settings.maxStep = static_cast<double>(step_dist(generator));
            }
            else {
                settings.maxStep /= sqrt5;
            }
        }

        velocityX += windX;
        velocityY += windY;
        velocityX += (settings.gravity * (settings.endX - settings.startX)) / dist;
        velocityY += (settings.gravity * (settings.endY - settings.startY)) / dist;

        if (std::hypot(velocityX, velocityY) > settings.maxStep) {
            randomDist = settings.maxStep / 2.0 + std::floor(rand2(generator) / 2.0 * settings.maxStep / 5.0); // 近似 TS 的随机逻辑
            // 为了更精确还原 TS: Math.floor((Math.random() * Math.round(settings.maxStep)) / 2)
            std::uniform_int_distribution<int> max_step_dist(0, static_cast<int>(std::round(settings.maxStep)));
            randomDist = settings.maxStep / 2.0 + static_cast<double>(max_step_dist(generator)) / 2.0;

            veloMag = std::hypot(velocityX, velocityY);
            velocityX = (velocityX / veloMag) * randomDist;
            velocityY = (velocityY / veloMag) * randomDist;
        }

        oldX = std::round(settings.startX);
        oldY = std::round(settings.startY);
        settings.startX += velocityX;
        settings.startY += velocityY;
        dist = std::hypot(settings.endX - settings.startX, settings.endY - settings.startY);
        newX = std::round(settings.startX);
        newY = std::round(settings.startY);

        step = std::hypot(settings.startX - oldX, settings.startY - oldY);
        int wait = static_cast<int>(std::round(waitDiff * (step / settings.maxStep) + settings.minWait));
        currentWait += wait;

        if (static_cast<int>(oldX) != static_cast<int>(newX) || static_cast<int>(oldY) != static_cast<int>(newY)) {
            points.push_back({ static_cast<int>(newX), static_cast<int>(newY), currentWait });
        }
    }

    int endX = static_cast<int>(std::round(settings.endX));
    int endY = static_cast<int>(std::round(settings.endY));

    if (endX != static_cast<int>(newX) || endY != static_cast<int>(newY)) {
        //co_yield{ static_cast<int>(newX), static_cast<int>(newY), currentWait };
        points.push_back({ static_cast<int>(newX), static_cast<int>(newY), currentWait });
    }
    //std::vector<MousePoint> result;
    return points;
}

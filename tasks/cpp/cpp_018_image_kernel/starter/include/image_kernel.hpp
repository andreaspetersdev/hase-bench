#pragma once
#include <cstddef>
#include <cstdint>
struct ImageView { const std::uint8_t* data; std::size_t width,height,stride; };
struct MutableImageView { std::uint8_t* data; std::size_t width,height,stride; };
void box_blur_3x3(ImageView source, MutableImageView output);

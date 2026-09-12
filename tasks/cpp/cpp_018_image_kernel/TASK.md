# CPP-018 — Stride-aware image kernel

Implement `box_blur_3x3` declared in `include/image_kernel.hpp` using C++20.
Images are 8-bit grayscale, with `stride >= width`; only the first `width`
bytes of each row are pixels. Output dimensions and stride equal input values.

For every output pixel, average the valid input pixels in its 3×3 neighborhood,
rounding down. This is clamp-free border handling: corners use four samples,
edge non-corners use six, and interior pixels use nine. `source` and `output`
may not overlap; throw `std::invalid_argument` when their active byte ranges
overlap or dimensions/strides are invalid. Padding bytes must not be read or
written. Empty images are valid and unchanged.

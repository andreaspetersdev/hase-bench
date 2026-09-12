#include "image_kernel.hpp"
#include <array>
#include <cassert>
int main(){std::array<unsigned char,9>in{0,0,0,0,9,0,0,0,0},out{};box_blur_3x3({in.data(),3,3,3},{out.data(),3,3,3});assert(out[4]==1&&out[0]==2);}

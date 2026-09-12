#include "image_kernel.hpp"
#include <array>
#include <cassert>
#include <stdexcept>
int main(){std::array<unsigned char,15> in{1,2,3,99,99,4,5,6,99,99,7,8,9,99,99};std::array<unsigned char,15>out{};out[3]=out[4]=out[8]=out[9]=out[13]=out[14]=77;box_blur_3x3({in.data(),3,3,5},{out.data(),3,3,5});assert(out[0]==3&&out[6]==5&&out[12]==7&&out[3]==77&&out[14]==77);std::array<unsigned char,4>row{0,9,0,0},row_out{};box_blur_3x3({row.data(),4,1,4},{row_out.data(),4,1,4});assert(row_out[0]==4&&row_out[1]==3&&row_out[2]==3&&row_out[3]==0);bool bad=false;try{box_blur_3x3({in.data(),3,3,2},{out.data(),3,3,5});}catch(const std::invalid_argument&){bad=true;}assert(bad);bad=false;try{box_blur_3x3({in.data(),3,3,5},{out.data(),2,3,5});}catch(const std::invalid_argument&){bad=true;}assert(bad);bad=false;try{box_blur_3x3({in.data(),3,3,5},{in.data(),3,3,5});}catch(const std::invalid_argument&){bad=true;}assert(bad);}

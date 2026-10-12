#include <iostream>
#include <cmath>
#include <vector>

struct Vec3 { 
    float x,y,z;
    Vec3 operator+(const Vec3& o) const { return {x+o.x, y+o.y, z+o.z}; }
    Vec3 operator-(const Vec3& o) const { return {x-o.x, y-o.y, z-o.z}; }
    Vec3 operator*(float s) const { return {x*s, y*s, z*s}; }
    Vec3 cross(const Vec3& o) const { return {y*o.z - z*o.y, z*o.x - x*o.z, x*o.y - y*o.x}; }
    float dot(const Vec3& o) const { return x*o.x + y*o.y + z*o.z; }
    Vec3 normalize() const {
        float l = std::sqrt(x*x+y*y+z*z);
        if (l < 1e-6) return {1,0,0};
        return {x/l, y/l, z/l};
    }
};

int main() {
    Vec3 u = {0,0,1};
    Vec3 w = {1,0,0};
    std::cout << "Done\n";
    return 0;
}

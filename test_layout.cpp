#include <iostream>
#include <cmath>
#include <vector>
#include <string>

struct Vec3 { 
    float x,y,z;
    Vec3 operator+(const Vec3& o) const { return {x+o.x, y+o.y, z+o.z}; }
    Vec3 operator-(const Vec3& o) const { return {x-o.x, y-o.y, z-o.z}; }
    Vec3 operator*(float s) const { return {x*s, y*s, z*s}; }
    Vec3 cross(const Vec3& o) const { return {y*o.z - z*o.y, z*o.x - x*o.z, x*o.y - y*o.x}; }
    float dot(const Vec3& o) const { return x*o.x + y*o.y + z*o.z; }
    Vec3 normalize() const {
        float l = std::sqrt(x*x+y*y+z*z);
        if (l < 1e-6f) return {1.0f,0.0f,0.0f};
        return {x/l, y/l, z/l};
    }
};

std::vector<Vec3> generar_tetraedro(Vec3 in_bond) {
    std::vector<Vec3> res;
    Vec3 u = in_bond.normalize();
    Vec3 w = {1,0,0};
    if (std::abs(u.x) > 0.9f) w = {0,1,0};
    w = (w - u * u.dot(w)).normalize(); 
    
    float c120 = -0.5f;
    float s120 = std::sqrt(3.0f)/2.0f;
    Vec3 cross = u.cross(w);
    
    Vec3 w1 = w;
    Vec3 w2 = w * c120 + cross * s120;
    Vec3 w3 = w * c120 - cross * s120;
    
    float ct = -1.0f/3.0f;
    float st = std::sqrt(8.0f/9.0f);
    
    res.push_back(u * ct + w1 * st);
    res.push_back(u * ct + w2 * st);
    res.push_back(u * ct + w3 * st);
    return res;
}

std::vector<Vec3> generar_trigonal(Vec3 in_bond) {
    std::vector<Vec3> res;
    Vec3 u = in_bond.normalize();
    Vec3 w = {1,0,0};
    if (std::abs(u.x) > 0.9f) w = {0,1,0};
    w = (w - u * u.dot(w)).normalize(); 
    
    float c120 = -0.5f;
    float s120 = std::sqrt(3.0f)/2.0f;
    Vec3 cross = u.cross(w);
    
    Vec3 w1 = w * c120 + cross * s120;
    Vec3 w2 = w * c120 - cross * s120;
    
    res.push_back(w1); // Wait, trigonal planar is in the plane of u and w!
    return res;
}

int main() {
    std::cout << "Done\n";
    return 0;
}

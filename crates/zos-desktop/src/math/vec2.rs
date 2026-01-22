//! 2D vector type for positions and offsets

use serde::{Deserialize, Serialize};

/// 2D vector for positions and offsets
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    /// Zero vector
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    /// Create a new vector
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Distance to another point
    #[inline]
    pub fn distance(self, other: Vec2) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Squared distance to another point (avoids sqrt)
    #[inline]
    pub fn distance_squared(self, other: Vec2) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }

    /// Length of the vector
    #[inline]
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// Squared length of the vector (avoids sqrt)
    #[inline]
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Normalize the vector (make it unit length)
    #[inline]
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len > 0.0 {
            Self::new(self.x / len, self.y / len)
        } else {
            Self::ZERO
        }
    }

    /// Linear interpolation between two vectors
    #[inline]
    pub fn lerp(a: Vec2, b: Vec2, t: f32) -> Vec2 {
        Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
    }
}

impl std::ops::Add for Vec2 {
    type Output = Vec2;
    #[inline]
    fn add(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Vec2;
    #[inline]
    fn sub(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x - other.x, self.y - other.y)
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn mul(self, s: f32) -> Vec2 {
        Vec2::new(self.x * s, self.y * s)
    }
}

impl std::ops::Div<f32> for Vec2 {
    type Output = Vec2;
    #[inline]
    fn div(self, s: f32) -> Vec2 {
        Vec2::new(self.x / s, self.y / s)
    }
}

impl std::ops::Neg for Vec2 {
    type Output = Vec2;
    #[inline]
    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec2_operations() {
        let a = Vec2::new(1.0, 2.0);
        let b = Vec2::new(3.0, 4.0);

        // Add
        let sum = a + b;
        assert!((sum.x - 4.0).abs() < 0.001);
        assert!((sum.y - 6.0).abs() < 0.001);

        // Sub
        let diff = b - a;
        assert!((diff.x - 2.0).abs() < 0.001);
        assert!((diff.y - 2.0).abs() < 0.001);

        // Mul
        let scaled = a * 2.0;
        assert!((scaled.x - 2.0).abs() < 0.001);
        assert!((scaled.y - 4.0).abs() < 0.001);

        // Div
        let divided = b / 2.0;
        assert!((divided.x - 1.5).abs() < 0.001);
        assert!((divided.y - 2.0).abs() < 0.001);

        // Neg
        let neg = -a;
        assert!((neg.x - (-1.0)).abs() < 0.001);
        assert!((neg.y - (-2.0)).abs() < 0.001);
    }

    #[test]
    fn test_vec2_distance() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(3.0, 4.0);
        assert!((a.distance(b) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_vec2_length() {
        let v = Vec2::new(3.0, 4.0);
        assert!((v.length() - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_vec2_normalize() {
        let v = Vec2::new(3.0, 4.0);
        let n = v.normalize();
        assert!((n.length() - 1.0).abs() < 0.001);
        assert!((n.x - 0.6).abs() < 0.001);
        assert!((n.y - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_vec2_lerp() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(10.0, 20.0);
        let mid = Vec2::lerp(a, b, 0.5);
        assert!((mid.x - 5.0).abs() < 0.001);
        assert!((mid.y - 10.0).abs() < 0.001);
    }
}

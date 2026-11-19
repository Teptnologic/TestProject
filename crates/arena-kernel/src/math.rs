//! 2D maths. Anything beyond `+ - * /` and `sqrt` goes through `libm` so that
//! native and WASM builds produce identical results.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

// Plain methods rather than operator traits, so call sites need no imports.
#[allow(clippy::should_implement_trait)]
impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub fn new(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }

    pub fn add(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x + o.x, self.y + o.y)
    }

    pub fn sub(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x - o.x, self.y - o.y)
    }

    pub fn scale(self, k: f64) -> Vec2 {
        Vec2::new(self.x * k, self.y * k)
    }

    pub fn dot(self, o: Vec2) -> f64 {
        self.x * o.x + self.y * o.y
    }

    pub fn len(self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn dist(self, o: Vec2) -> f64 {
        self.sub(o).len()
    }

    /// Unit vector, or `fallback` when the vector is (almost) zero.
    pub fn normalized_or(self, fallback: Vec2) -> Vec2 {
        let l = self.len();
        if l < 1e-9 {
            fallback
        } else {
            self.scale(1.0 / l)
        }
    }

    /// Moves towards `to` by at most `step`. Returns the new point and whether it arrived.
    pub fn step_towards(self, to: Vec2, step: f64) -> (Vec2, bool) {
        let d = self.dist(to);
        if d <= step {
            (to, true)
        } else {
            (self.add(to.sub(self).scale(step / d)), false)
        }
    }

    pub fn clamp(self, max: Vec2) -> Vec2 {
        Vec2::new(self.x.max(0.0).min(max.x), self.y.max(0.0).min(max.y))
    }
}

/// Closest approach of point `p` to segment `a..b`: (distance, position along the segment 0..1).
pub fn segment_distance(a: Vec2, b: Vec2, p: Vec2) -> (f64, f64) {
    let ab = b.sub(a);
    let len2 = ab.dot(ab);
    let t = if len2 < 1e-12 {
        0.0
    } else {
        (p.sub(a).dot(ab) / len2).clamp(0.0, 1.0)
    };
    (a.add(ab.scale(t)).dist(p), t)
}

pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}

pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}

pub fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

pub fn pow(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}

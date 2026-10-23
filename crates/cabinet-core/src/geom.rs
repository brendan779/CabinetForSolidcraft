//! Minimal geometry: points and axis-aligned boxes in millimetres.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn get(self, axis: Axis) -> f64 {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
            Axis::Z => self.z,
        }
    }

    pub fn with(mut self, axis: Axis, v: f64) -> Self {
        match axis {
            Axis::X => self.x = v,
            Axis::Y => self.y = v,
            Axis::Z => self.z = v,
        }
        self
    }

    pub fn to_array(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

impl std::ops::Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub const ALL: [Axis; 3] = [Axis::X, Axis::Y, Axis::Z];

    /// The two axes that are not `self`, in X, Y, Z order.
    pub fn others(self) -> [Axis; 2] {
        match self {
            Axis::X => [Axis::Y, Axis::Z],
            Axis::Y => [Axis::X, Axis::Z],
            Axis::Z => [Axis::X, Axis::Y],
        }
    }
}

/// An axis-aligned box given by its minimum corner and its size.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Aabb {
    pub min: Vec3,
    pub size: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, size: Vec3) -> Self {
        Self { min, size }
    }

    pub fn max(&self) -> Vec3 {
        self.min + self.size
    }

    pub fn center(&self) -> Vec3 {
        Vec3::new(
            self.min.x + self.size.x / 2.0,
            self.min.y + self.size.y / 2.0,
            self.min.z + self.size.z / 2.0,
        )
    }

    /// True when the boxes share volume (touching faces do not count).
    pub fn overlaps(&self, o: &Aabb) -> bool {
        const EPS: f64 = 1e-6;
        Axis::ALL.iter().all(|&a| {
            self.min.get(a) < o.max().get(a) - EPS && o.min.get(a) < self.max().get(a) - EPS
        })
    }

    pub fn volume(&self) -> f64 {
        self.size.x * self.size.y * self.size.z
    }
}

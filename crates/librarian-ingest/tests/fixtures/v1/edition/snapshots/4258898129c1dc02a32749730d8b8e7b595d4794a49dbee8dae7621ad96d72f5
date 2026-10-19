use crate::{camera::Ray, Hit, Shape};

pub struct Sphere {
    pub radius: f64,
}

impl Sphere {
    pub fn unit() -> Self {
        Sphere { radius: 1.0 }
    }
}

impl Shape for Sphere {
    fn intersect(&self, ray: &Ray) -> Hit {
        let along = ray.direction.2 * self.radius;
        Hit::At(along)
    }
}

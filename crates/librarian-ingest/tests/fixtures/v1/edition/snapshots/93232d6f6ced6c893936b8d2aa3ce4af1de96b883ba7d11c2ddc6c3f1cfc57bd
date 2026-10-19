//! Fixture v1 do librarian-ingest. Não é compilada; só é extraída.
pub mod camera;
pub mod shapes;

pub const MAX_DEPTH: usize = 5;
pub static NAME: &str = "fixture";

pub type Scalar = f64;

pub enum Hit {
    Miss,
    At(Scalar),
}

pub union Bits {
    int: u64,
    float: f64,
}

pub trait Shape {
    fn intersect(&self, ray: &camera::Ray) -> Hit;
    fn name(&self) -> &str {
        "shape"
    }
}

pub fn render(camera: &camera::Camera, shape: &dyn Shape) -> Vec<Hit> {
    let ray = camera.ray_from_pixel(0, 0);
    let hit = shape.intersect(&ray);
    vec![hit]
}

#[cfg(any())]
fn disabled() {}

macro_rules! make {
    () => {
        fn generated() {}
    };
}
make!();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_returns_one_hit() {
        let camera = camera::Camera::new(4, 3);
        assert_eq!(render(&camera, &shapes::Sphere::unit()).len(), 1);
    }
}

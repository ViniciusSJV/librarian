pub struct Point(pub f64, pub f64, pub f64);

pub struct Ray {
    pub origin: Point,
    pub direction: Point,
}

pub struct Camera {
    pub width: usize,
    pub height: usize,
}

impl Camera {
    pub fn new(width: usize, height: usize) -> Self {
        Camera { width, height }
    }

    pub fn ray_from_pixel(&self, x: usize, y: usize) -> Ray {
        let origin = Point(0.0, 0.0, 0.0);
        let pixel = Point(x as f64, y as f64, -1.0);
        let direction = normalize(subtract(pixel, &origin));
        Ray { origin, direction }
    }
}

fn subtract(a: Point, b: &Point) -> Point {
    Point(a.0 - b.0, a.1 - b.1, a.2 - b.2)
}

fn normalize(p: Point) -> Point {
    let len = (p.0 * p.0 + p.1 * p.1 + p.2 * p.2).sqrt();
    Point(p.0 / len, p.1 / len, p.2 / len)
}

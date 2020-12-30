use super::{ground_vec2, Art, Rot};
use glam::{vec3, Vec2, Vec3, Mat4};

pub struct Cars {
    cars: Vec<Car>,
    track_travelled: f32,
}

impl Default for Cars {
    fn default() -> Self {
        Cars {
            cars: vec![
                Car {
                    art: Art::Train,
                    length: 8.0,
                    axles: [
                        Some(Axle { wheel_radius: 1.5, offset: 2.0 }),
                        Some(Axle { wheel_radius: 1.1, offset: 6.0 }),
                    ],
                    ..Default::default()
                },
                Car {
                    art: Art::Cart,
                    length: 4.76,
                    axles: [Some(Axle { wheel_radius: 1.1, offset: 3.0 }), None],
                    gun: Some(Gun { offset: -1.205 }),
                },
            ],
            track_travelled: 0.0,
        }
    }
}

impl super::Stage {
    pub fn draw_train(&mut self, rq: &mut super::RenderQueue) {
        self.train.track_travelled += 0.8;
        let dist = self.train.track_travelled;
        let mut length_so_far = 0.0;

        for car in &self.train.cars {
            let front = self.track_point(dist - length_so_far);
            length_so_far += car.length;
            let back = self.track_point(dist - length_so_far);

            let to_back = Rot::from_vec2(back - front);
            rq.draw(car.art, front, to_back);

            for &Axle { offset, wheel_radius } in car.axles.iter().filter_map(|x| x.as_ref()) {
                use std::f32::consts::{FRAC_PI_2, PI, TAU};

                for &(pitch, out_dir) in &[(0.0, -1.0), (PI, 1.0)] {
                    let out = to_back.vec2().perp() * out_dir * 1.4;
                    let Vec2 { x, y: z } = front + to_back.vec2() * offset + out;
                    rq.draw_mat4(
                        Art::Wheel,
                        Mat4::from_translation(vec3(x, wheel_radius, z))
                            * Mat4::from_rotation_y(pitch + FRAC_PI_2 - to_back.0)
                            * Mat4::from_rotation_x(dist / TAU * wheel_radius * out_dir - pitch / 2.0)
                            * Mat4::from_scale(Vec3::splat(wheel_radius)),
                    )
                }
            }

            if let Some(gun) = &car.gun {
                rq.draw(Art::Gun, front - to_back.vec2() * gun.offset, to_back);
            }

            length_so_far += 2.109;
        }

        self.cam_origin = ground_vec2(self.track_point(dist));
        self.cam_offset = {
            let Vec2 { x, y } = self.track_point(dist - length_so_far - 20.0);
            vec3(x, 20.0, y) - self.cam_origin
        };
    }
}

struct Car {
    length: f32,
    art: Art,
    axles: [Option<Axle>; 2],
    gun: Option<Gun>,
}

impl Default for Car {
    fn default() -> Self {
        Car { length: 5.0, art: Art::Cart, axles: [None, None], gun: None }
    }
}

struct Gun {
    offset: f32,
}

struct Axle {
    offset: f32,
    wheel_radius: f32,
}

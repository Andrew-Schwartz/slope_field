use std::fs::File;
use std::io::Read;

use fasteval::{Compiler, eval_compiled_ref, Instruction, Parser, Slab};
use fasteval::evaler::Evaler;
use ggez::{Context, ContextBuilder, event, GameResult, graphics};
use ggez::event::EventHandler;
use ggez::graphics::{BLACK, Color};
use ggez::input::mouse;
use ggez::mint::Point2;

struct State {
    dydt: Instruction,
    slab: Slab,
    dt: f32,
    t_min: f32,
    t_max: f32,
    t_div: i32,
    y_min: f32,
    y_max: f32,
    y_div: i32,
    pl: Point2<f32>,
    pr: Point2<f32>,
}

impl  State {
    fn new(eq: Instruction, slab: Slab) -> State {
        State {
            dydt: eq,
            slab,
            dt: 0.01,
            t_min: -10.0,
            t_max: 10.0,
            t_div: 40,
            y_min: -10.0,
            y_max: 10.0,
            y_div: 32,
            pl: Point2 { x: 1.0, y: 1.0 },
            pr: Point2 { x: 1.0, y: 1.0 },
        }
    }

    fn calculate<F>(&self, mut f: F) -> Result<f64, fasteval::error::Error>
        where F: FnMut(&str, Vec<f64>) -> Option<f64> {
        let x = eval_compiled_ref!(&self.dydt, &self.slab, &mut f);
        Ok(x)
    }
}

impl  EventHandler for State {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.pl = from_scrn(self, &mouse::position(ctx));
        self.pr = self.pl;

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let (w, h) = (800.0, 600.0);
        let (x_spacing, y_spacing) = (w / self.t_div as f32, h / self.y_div as f32);

        graphics::clear(ctx, graphics::Color::from_rgb(255, 255, 255));

        let mut builder = graphics::MeshBuilder::new();

        for j in 0..=self.y_div {
            for i in 0..=self.t_div {
                let t = (self.t_max - self.t_min) as f32 / self.t_div as f32 * i as f32 + self.t_min as f32;
                let y = (self.y_max - self.y_min) as f32 / self.y_div as f32 * j as f32 + self.y_min as f32;

                let cb = |name: &str, _args: Vec<f64>| -> Option<f64> {
                    match name {
                        "t" => Some(t as f64),
                        "y" => Some(y as f64),
                        _ => None
                    }
                };
                let slope = self.calculate(cb).unwrap() as f32;

                let Point2 { x, y } = to_scrn(self, &Point2 { x: t, y });

                let theta = slope.atan();

                let p1 = Point2 {
                    x: x - theta.cos() * x_spacing / 4.0,
                    y: y + theta.sin() * y_spacing / 4.0,
                };
                let p2 = Point2 {
                    x: x + theta.cos() * x_spacing / 4.0,
                    y: y - theta.sin() * y_spacing / 4.0,
                };

                builder.line(&[p1, p2], 1.2, BLACK)?;
            }
        }

        let mut left = self.pl.clone();
        let mut right = self.pr.clone();

        while in_bounds(&left, self) || in_bounds(&right, self) {
            if in_bounds(&left, self) {
                let cb = |name: &str, _args: Vec<f64>| -> Option<f64> {
                    match name {
                        "t" => Some(left.x as f64),
                        "y" => Some(left.y as f64),
                        _ => None
                    }
                };
                let delta = self.calculate(cb).unwrap() as f32;

                let new = Point2 {
                    x: left.x - self.dt,
                    y: left.y - delta * self.dt,
                };
                builder.line(
                    &[to_scrn(self, &left), to_scrn(self, &new)],
                    1.5,
                    Color::from_rgb(255, 0, 0),
                )?;

                left = new;
            }
            if in_bounds(&right, self) {
                let cb = |name: &str, _args: Vec<f64>| -> Option<f64> {
                    match name {
                        "t" => Some(right.x as f64),
                        "y" => Some(right.y as f64),
                        _ => None
                    }
                };
                let delta = self.calculate(cb).unwrap() as f32;

                let new = Point2 {
                    x: right.x + self.dt,
                    y: right.y + delta * self.dt,
                };
                builder.line(
                    &[to_scrn(self, &right), to_scrn(self, &new)],
                    1.0,
                    Color::from_rgb(255, 0, 0),
                )?;

                right = new;
            }
        }

        match builder.build(ctx) {
            Ok(mesh) => graphics::draw(ctx, &mesh, graphics::DrawParam::default())?,
            Err(_) => {}
        }

        graphics::present(ctx)?;

        Ok(())
    }
}


fn in_bounds(pt: &Point2<f32>, state: &State) -> bool {
    let Point2 { x, y } = *pt;
    x >= state.t_min && x <= state.t_max && y >= state.y_min && y <= state.y_max
}

fn to_scrn(state: &State, pt: &Point2<f32>) -> Point2<f32> {
    Point2 {
        x: 400.0 + pt.x / (state.t_max - state.t_min) * 800.0,
        y: 300.0 - pt.y / (state.y_max - state.y_min) * 600.0,
    }
}

fn from_scrn(state: &State, pt: &Point2<f32>) -> Point2<f32> {
    Point2 {
        x: (pt.x - 400.0) / 800.0 * (state.t_max - state.t_min),
        y: -(pt.y - 300.0) / 600.0 * (state.y_max - state.y_min),
    }
}

fn main() {
    let eq1 = read_eq();

    let parser = Parser::new();
    let mut slab = Slab::new();

    let compiled = parser.parse(&eq1, &mut slab.ps)
        .unwrap()
        .from(&slab.ps)
        .compile(&slab.ps, &mut slab.cs);

    let state = &mut State::new(compiled, slab);

    let cb = ContextBuilder::new("", "");
    let (ref mut ctx, ref mut event_loop) = &mut cb.build().unwrap();

    event::run(ctx, event_loop, state).unwrap();
}

fn read_eq() -> String {
    let mut file = File::open("eq.txt").unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    contents
}

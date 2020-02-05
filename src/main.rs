use std::fs::File;
use std::io::Read;

use fasteval::{Compiler, eval_compiled_ref, Instruction, Parser, Slab};
use fasteval::evaler::Evaler;
use ggez::{Context, ContextBuilder, event, GameResult, graphics};
use ggez::event::EventHandler;
use ggez::graphics::{BLACK, Color};
use ggez::input::keyboard::KeyCode;
use ggez::input::mouse;
use ggez::mint::Point2;

struct State {
    dydt: Instruction,
    slab: Slab,
    parser: Parser,
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

impl State {
    fn new() -> State {
        State {
            dydt: Instruction::IConst(1.0),
            slab: Slab::new(),
            parser: Parser::new(),
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

//    fn read_eq(&mut self) {
//        let mut file = File::open("eq.txt").unwrap();
//        let mut contents = String::new();
//        file.read_to_string(&mut contents).unwrap();
//
//        self.dydt = self.parser.parse(&contents, &mut self.slab.ps)
//            .unwrap()
//            .from(&self.slab.ps)
//            .compile(&self.slab.ps, &mut self.slab.cs);
//    }

    fn read_cfg(&mut self) {
        let mut file = File::open("cfg.txt").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        contents.lines()
            .map(|line| {
                let mut split = line.split(":");
                (split.next().unwrap(), split.next().unwrap())
            })
            .for_each(|(var, val)| {
                match var {
                    "t min" => self.t_min = val.trim().parse().unwrap_or(self.t_min),
                    "t max" => self.t_max = val.trim().parse().unwrap_or(self.t_max),
                    "t div" => self.t_div = val.trim().parse().unwrap_or(self.t_div),
                    "y min" => self.y_min = val.trim().parse().unwrap_or(self.y_min),
                    "y max" => self.y_max = val.trim().parse().unwrap_or(self.y_max),
                    "y div" => self.y_div = val.trim().parse().unwrap_or(self.y_div),
                    "eq" => self.dydt = self.parser.parse(&val, &mut self.slab.ps)
                        .unwrap()
                        .from(&self.slab.ps)
                        .compile(&self.slab.ps, &mut self.slab.cs),
                    _ => println!("{}", var),
                }
            });
    }
}

impl EventHandler for State {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.pl = mouse::position(ctx).from_scrn(self);
        self.pr = self.pl;

        let new_eq = ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Return);
        if new_eq {
//            self.read_eq();
            self.read_cfg();
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let (w, h) = (800.0, 600.0);
        let (x_spacing, y_spacing) = (w / self.t_div as f32, h / self.y_div as f32);

        graphics::clear(ctx, graphics::Color::from_rgb(255, 255, 255));

        let mut builder = graphics::MeshBuilder::new();

        builder.line(&[
            Point2 { x: self.t_min, y: 0.0 }.to_scrn(self),
            Point2 { x: self.t_max, y: 0.0 }.to_scrn(self)
        ], 1.0, BLACK)?;

        builder.line(&[
            Point2 { x: 0.0, y: self.t_min }.to_scrn(self),
            Point2 { x: 0.0, y: self.y_max }.to_scrn(self)
        ], 1.0, BLACK)?;

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

                let Point2 { x, y } = Point2 { x: t, y }.to_scrn(self);

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

        while (left.x >= self.t_min && left.x <= self.t_max) || (right.x >= self.t_min && right.x <= self.t_max) {
            if left.x >= self.t_min && left.x <= self.t_max {
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
                    &[left.to_scrn(self), new.to_scrn(self)],
                    1.5,
                    Color::from_rgb(255, 0, 0),
                )?;

                left = new;
            }
            if right.x >= self.t_min && right.x <= self.t_max {
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
                    &[right.to_scrn(self), new.to_scrn(self)],
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

trait PointScale {
    fn to_scrn(&self, state: &State) -> Self;

    fn from_scrn(&self, state: &State) -> Self;
}

impl PointScale for Point2<f32> {
    fn to_scrn(&self, state: &State) -> Self {
        Point2 {
            x: 400.0 + self.x / (state.t_max - state.t_min) * 800.0,
            y: 300.0 - self.y / (state.y_max - state.y_min) * 600.0,
        }
    }

    fn from_scrn(&self, state: &State) -> Self {
        Point2 {
            x: (self.x - 400.0) / 800.0 * (state.t_max - state.t_min),
            y: -(self.y - 300.0) / 600.0 * (state.y_max - state.y_min),
        }
    }
}

fn main() {
    let state = &mut State::new();
//    state.read_eq();
    state.read_cfg();

    let cb = ContextBuilder::new("", "");
    let (ref mut ctx, ref mut event_loop) = &mut cb.build().unwrap();

    event::run(ctx, event_loop, state).unwrap();
}
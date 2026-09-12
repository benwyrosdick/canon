use bevy::prelude::*;

use crate::{
    physics::*,
    terrain::{MapSize, Rng, Terrain},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Handoff,
    Aiming,
    Flying,
    Resolving(f32),
    Finished(Option<usize>),
}

#[derive(Clone, Copy)]
pub struct Cannon {
    pub position: Vec3,
    pub yaw: f32,
    pub elevation: f32,
    pub power: f32,
    pub health: f32,
}

impl Cannon {
    pub fn direction(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.elevation.cos(),
            self.elevation.sin(),
            self.yaw.sin() * self.elevation.cos(),
        )
    }

    pub fn muzzle(&self) -> Vec3 {
        self.position + Vec3::Y * 0.5 + self.direction() * 3.2
    }
}

pub enum Effect {
    Fire(Vec3),
    Blast(Vec3),
}

#[derive(Resource)]
pub struct Game {
    pub seed: u64,
    pub size: MapSize,
    /// Applied by New Map; selecting a size never interrupts the current match.
    pub next_size: MapSize,
    pub cannons: [Cannon; 2],
    pub active: usize,
    pub round: u32,
    pub wind: Vec3,
    pub phase: Phase,
    pub paused: bool,
    pub muted: bool,
    pub ball: Option<Ball>,
    pub trail: Vec<Vec3>,
    pub message: String,
    pub effects: Vec<Effect>,
    rng: Rng,
}

impl Game {
    pub fn new(seed: u64, terrain: &Terrain) -> Self {
        let spawns = terrain.size.spawns();
        let cannons = std::array::from_fn(|i| {
            let spawn = spawns[i];
            let delta = spawns[1 - i] - spawn;
            Cannon {
                position: Vec3::new(
                    spawn.x,
                    terrain.height(spawn.x, spawn.y).unwrap() + 1.2,
                    spawn.y,
                ),
                yaw: delta.y.atan2(delta.x),
                elevation: 45.0_f32.to_radians(),
                power: 32.0 * terrain.size.scale().sqrt(),
                health: 100.0,
            }
        });
        let mut game = Self {
            seed,
            size: terrain.size,
            next_size: terrain.size,
            cannons,
            active: 0,
            round: 1,
            wind: Vec3::ZERO,
            phase: Phase::Handoff,
            paused: false,
            muted: false,
            ball: None,
            trail: Vec::new(),
            message: "A fresh battlefield. Make your first shot count.".into(),
            effects: Vec::new(),
            rng: Rng::new(seed ^ 0xCA7707),
        };
        game.roll_wind();
        game
    }

    fn roll_wind(&mut self) {
        let angle = self.rng.range(0.0, std::f32::consts::TAU);
        let speed = self.rng.range(0.0, 14.0);
        self.wind = Vec3::new(angle.cos(), 0.0, angle.sin()) * speed;
    }

    pub fn fire(&mut self) {
        if self.paused || self.phase != Phase::Aiming {
            return;
        }
        let cannon = self.cannons[self.active];
        self.ball = Some(Ball {
            position: cannon.muzzle(),
            velocity: cannon.direction() * cannon.power,
            age: 0.0,
        });
        self.trail.clear();
        self.trail.push(cannon.muzzle());
        self.effects.push(Effect::Fire(cannon.muzzle()));
        self.phase = Phase::Flying;
        self.message = "Shot away!".into();
    }

    fn finish_turn(&mut self) {
        self.phase = match (self.cannons[0].health <= 0.0, self.cannons[1].health <= 0.0) {
            (true, true) => Phase::Finished(None),
            (true, false) => Phase::Finished(Some(1)),
            (false, true) => Phase::Finished(Some(0)),
            (false, false) => {
                self.active = 1 - self.active;
                if self.active == 0 {
                    self.round += 1;
                    self.roll_wind();
                }
                Phase::Handoff
            }
        };
    }
}

#[derive(Component, Clone, Copy)]
pub enum Action {
    Primary,
    Restart,
    SelectSize(MapSize),
}

pub fn fresh_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

pub fn input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut game: ResMut<Game>,
    mut terrain: ResMut<Terrain>,
    mut meshes: ResMut<Assets<Mesh>>,
    buttons: Query<(&Interaction, &Action), Changed<Interaction>>,
) {
    let mut primary = keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter);
    let mut restart = keys.just_pressed(KeyCode::KeyR);
    for (interaction, action) in &buttons {
        if *interaction == Interaction::Pressed {
            match action {
                Action::Primary => primary = true,
                Action::Restart => restart = true,
                Action::SelectSize(size) => game.next_size = *size,
            }
        }
    }
    if restart {
        let (seed, size) = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight)
        {
            (game.seed, game.size)
        } else {
            (fresh_seed(), game.next_size)
        };
        let mut next = Terrain::new(seed, size);
        next.mesh = terrain.mesh.clone();
        if let Some(mesh) = meshes.get_mut(&terrain.mesh) {
            *mesh = next.build_mesh();
        }
        let muted = game.muted;
        *game = Game::new(seed, &next);
        game.muted = muted;
        *terrain = next;
        return;
    }
    if keys.just_pressed(KeyCode::KeyM) {
        game.muted = !game.muted;
    }
    if keys.just_pressed(KeyCode::Escape) {
        game.paused = !game.paused;
    }
    if game.paused {
        return;
    }
    if primary {
        if game.phase == Phase::Handoff {
            game.phase = Phase::Aiming;
        } else {
            game.fire();
        }
    }
    if game.phase != Phase::Aiming {
        return;
    }
    let axis =
        |positive, negative| f32::from(keys.pressed(positive)) - f32::from(keys.pressed(negative));
    let precision = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        0.2
    } else {
        1.0
    };
    let dt = time.delta_secs().min(0.05) * precision;
    let active = game.active;
    let cannon = &mut game.cannons[active];
    // A decreases bearing; D increases it.
    cannon.yaw += axis(KeyCode::KeyD, KeyCode::KeyA) * dt * 0.55;
    cannon.yaw = cannon.yaw.rem_euclid(std::f32::consts::TAU);
    cannon.elevation = (cannon.elevation + axis(KeyCode::KeyW, KeyCode::KeyS) * dt * 0.45)
        .clamp(5.0_f32.to_radians(), 85.0_f32.to_radians());
    cannon.power =
        (cannon.power + axis(KeyCode::KeyE, KeyCode::KeyQ) * dt * 14.0).clamp(15.0, 52.0);
}

pub fn simulate(
    time: Res<Time<Fixed>>,
    mut game: ResMut<Game>,
    mut terrain: ResMut<Terrain>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if game.paused {
        return;
    }
    let dt = time.delta_secs();
    if let Phase::Resolving(remaining) = game.phase {
        for cannon in &mut game.cannons {
            let ground = terrain
                .height(cannon.position.x, cannon.position.z)
                .unwrap()
                + 1.2;
            cannon.position.y = (cannon.position.y - 10.0 * dt).max(ground);
        }
        if remaining <= dt {
            game.finish_turn();
        } else {
            game.phase = Phase::Resolving(remaining - dt);
        }
        return;
    }
    if game.phase != Phase::Flying {
        return;
    }
    let Some(mut ball) = game.ball else {
        return;
    };
    let old = ball.step(dt, game.wind);
    let mut hit = terrain.sweep(old, ball.position).map(|t| (t, None));
    for (i, cannon) in game.cannons.iter().enumerate() {
        if let Some(t) = segment_sphere(old, ball.position, cannon.position, CANNON_RADIUS + 0.28)
            && hit.is_none_or(|(first, _)| t < first)
        {
            hit = Some((t, Some(i)));
        }
    }
    if let Some((fraction, direct)) = hit {
        let point = old.lerp(ball.position, fraction);
        let mut damage = [0.0; 2];
        for (i, cannon) in game.cannons.iter_mut().enumerate() {
            damage[i] = if direct == Some(i) {
                100.0
            } else {
                blast_damage(cannon.position.distance(point))
            };
            damage[i] = damage[i].min(cannon.health);
            cannon.health = (cannon.health - damage[i]).max(0.0);
        }
        terrain.crater(point, BLAST_RADIUS);
        if let Some(mesh) = meshes.get_mut(&terrain.mesh) {
            *mesh = terrain.build_mesh();
        }
        game.message = if direct.is_some() {
            "DIRECT HIT!".into()
        } else if damage.iter().any(|d| *d > 0.0) {
            format!(
                "Splash damage - Red -{:.0} / Blue -{:.0}",
                damage[0], damage[1]
            )
        } else {
            "Terrain hit. A new crater, a new possibility.".into()
        };
        game.trail.push(point);
        game.effects.push(Effect::Blast(point));
        game.ball = None;
        game.phase = Phase::Resolving(1.5);
    } else if ball.age > MAX_FLIGHT
        || ball.position.y < -20.0
        || ball.position.x.abs() > game.size.half() + 15.0
        || ball.position.z.abs() > game.size.half() + 15.0
    {
        game.ball = None;
        game.phase = Phase::Resolving(0.8);
        game.message = "Out of bounds. Adjust your aim and power next turn.".into();
    } else {
        if game
            .trail
            .last()
            .is_none_or(|p| p.distance(ball.position) > 0.7)
        {
            game.trail.push(ball.position);
        }
        game.ball = Some(ball);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_and_cannon_spacing_follow_the_active_size() {
        for size in MapSize::ALL {
            let mut app = simulation_app(size);
            {
                let mut game = app.world_mut().resource_mut::<Game>();
                assert!((game.cannons[0].position.x + 40.0 * size.scale()).abs() < 0.001);
                assert!((game.cannons[1].position.x - 40.0 * size.scale()).abs() < 0.001);
                game.phase = Phase::Flying;
                game.ball = Some(Ball {
                    position: Vec3::new(size.half() + 10.0, 100.0, 0.0),
                    velocity: Vec3::ZERO,
                    age: 0.0,
                });
            }
            app.update();
            assert_eq!(app.world().resource::<Game>().phase, Phase::Flying);
            app.world_mut()
                .resource_mut::<Game>()
                .ball
                .as_mut()
                .unwrap()
                .position
                .x = size.half() + 16.0;
            app.update();
            assert!(app.world().resource::<Game>().ball.is_none());
        }
    }

    #[test]
    fn size_selection_applies_on_new_map_and_replay_keeps_current_size_and_seed() {
        let mut app = App::new();
        let mut meshes = Assets::<Mesh>::default();
        let mut terrain = Terrain::new(42, MapSize::Small);
        terrain.mesh = meshes.add(terrain.build_mesh());
        let mut game = Game::new(42, &terrain);
        game.phase = Phase::Aiming;
        game.fire();
        app.insert_resource(game)
            .insert_resource(terrain)
            .insert_resource(meshes)
            .insert_resource(Time::<()>::default())
            .insert_resource(ButtonInput::<KeyCode>::default())
            .add_systems(Update, input);
        app.world_mut()
            .spawn((Action::SelectSize(MapSize::Large), Interaction::Pressed));
        app.update();
        let game = app.world().resource::<Game>();
        assert_eq!(game.size, MapSize::Small);
        assert_eq!(game.next_size, MapSize::Large);
        assert!(game.ball.is_some());

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.update();
        let game = app.world().resource::<Game>();
        let seed = game.seed;
        let wind = game.wind;
        let positions = game.cannons.map(|c| c.position);
        assert_eq!(game.size, MapSize::Large);
        assert_eq!(game.phase, Phase::Handoff);
        assert!(game.ball.is_none());
        assert_eq!(app.world().resource::<Terrain>().size, MapSize::Large);

        app.world_mut().resource_mut::<Game>().next_size = MapSize::Medium;
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::ShiftLeft);
            keys.press(KeyCode::KeyR);
        }
        app.update();
        let game = app.world().resource::<Game>();
        assert_eq!(
            (game.size, game.next_size, game.seed),
            (MapSize::Large, MapSize::Large, seed)
        );
        assert_eq!(game.wind, wind);
        assert_eq!(game.cannons.map(|c| c.position), positions);
    }

    fn simulation_app(size: MapSize) -> App {
        let mut app = App::new();
        let mut terrain = Terrain::new(42, size);
        let mut meshes = Assets::<Mesh>::default();
        terrain.mesh = meshes.add(terrain.build_mesh());
        let game = Game::new(42, &terrain);
        let mut time = Time::<Fixed>::from_hz(120.0);
        time.advance_by(std::time::Duration::from_secs_f64(1.0 / 120.0));
        app.insert_resource(time)
            .insert_resource(terrain)
            .insert_resource(meshes)
            .insert_resource(game)
            .add_systems(Update, simulate);
        app
    }

    #[test]
    fn direct_hit_deforms_ground_settles_cannon_and_ends_match_once() {
        let mut app = simulation_app(MapSize::Small);
        let target = app.world().resource::<Game>().cannons[1].position;
        {
            let mut game = app.world_mut().resource_mut::<Game>();
            game.phase = Phase::Flying;
            game.ball = Some(Ball {
                position: target + Vec3::new(-4.0, 0.5, 0.0),
                velocity: Vec3::X * 1000.0,
                age: 0.0,
            });
        }
        app.update();
        let game = app.world().resource::<Game>();
        assert_eq!(game.cannons[1].health, 0.0);
        assert_eq!(game.cannons[0].health, 100.0);
        assert!(game.ball.is_none());
        assert_eq!(game.effects.len(), 1);
        assert!(
            app.world()
                .resource::<Terrain>()
                .height(target.x, target.z)
                .unwrap()
                < target.y - 2.0
        );
        for _ in 0..200 {
            app.update();
        }
        let game = app.world().resource::<Game>();
        assert_eq!(game.phase, Phase::Finished(Some(0)));
        assert_eq!(game.effects.len(), 1);
        assert!(game.cannons[1].position.y < target.y);
    }

    #[test]
    fn pause_freezes_shot_and_miss_hands_off_without_damage() {
        let mut app = simulation_app(MapSize::Small);
        {
            let mut game = app.world_mut().resource_mut::<Game>();
            game.phase = Phase::Flying;
            game.paused = true;
            game.ball = Some(Ball {
                position: Vec3::new(MapSize::Small.half() + 20.0, 100.0, 0.0),
                velocity: Vec3::X * 30.0,
                age: 0.0,
            });
        }
        app.update();
        assert_eq!(app.world().resource::<Game>().ball.unwrap().age, 0.0);
        app.world_mut().resource_mut::<Game>().paused = false;
        for _ in 0..110 {
            app.update();
        }
        let game = app.world().resource::<Game>();
        assert_eq!(game.phase, Phase::Handoff);
        assert_eq!(game.active, 1);
        assert!(game.cannons.iter().all(|c| c.health == 100.0));
    }

    #[test]
    fn turns_require_ready_and_wind_is_shared_for_a_round() {
        let terrain = Terrain::new(1, MapSize::Small);
        let mut g = Game::new(1, &terrain);
        g.fire();
        assert!(g.ball.is_none());
        g.phase = Phase::Aiming;
        g.paused = true;
        g.fire();
        assert!(g.ball.is_none());
        g.paused = false;
        g.fire();
        assert_eq!(g.phase, Phase::Flying);
        assert!(g.ball.is_some());
        g.effects.clear();
        g.fire();
        assert!(g.effects.is_empty());
        let wind = g.wind;
        g.finish_turn();
        assert_eq!((g.active, g.round, g.phase), (1, 1, Phase::Handoff));
        assert_eq!(g.wind, wind);
        g.finish_turn();
        assert_eq!((g.active, g.round), (0, 2));
        assert_ne!(g.wind, wind);
    }

    #[test]
    fn winner_and_draw_do_not_start_another_turn() {
        let t = Terrain::new(2, MapSize::Small);
        for (health, expected) in [
            ([0.0, 100.0], Some(1)),
            ([100.0, 0.0], Some(0)),
            ([0.0, 0.0], None),
        ] {
            let mut g = Game::new(2, &t);
            for (c, h) in g.cannons.iter_mut().zip(health) {
                c.health = h;
            }
            g.finish_turn();
            assert_eq!(g.phase, Phase::Finished(expected));
            assert_eq!(g.round, 1);
        }
    }
}

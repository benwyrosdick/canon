use bevy::prelude::*;

use crate::{
    net::{self, Msg, Net, PlayMode},
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

    pub(crate) fn place_on_lane(&mut self, z: f32, reach: f32, terrain: &Terrain) {
        self.position.z = z.clamp(-reach, reach);
        if let Some(ground) = terrain.height(self.position.x, self.position.z) {
            self.position.y = ground + 1.2;
        }
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
        let spawns = terrain.spawns;
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

    pub fn restart_world(
        seed: u64,
        size: MapSize,
        game: &mut Game,
        terrain: &mut Terrain,
        meshes: &mut Assets<Mesh>,
    ) {
        let muted = game.muted;
        let next_size = game.next_size;
        let mut next = Terrain::new(seed, size);
        next.mesh = terrain.mesh.clone();
        if let Some(mesh) = meshes.get_mut(&terrain.mesh) {
            *mesh = next.build_mesh();
        }
        *game = Game::new(seed, &next);
        game.muted = muted;
        game.next_size = next_size;
        *terrain = next;
    }

    fn health(&self) -> [f32; 2] {
        [self.cannons[0].health, self.cannons[1].health]
    }

    fn cannon_y(&self) -> [f32; 2] {
        [self.cannons[0].position.y, self.cannons[1].position.y]
    }

    fn turn_msg(&self) -> Msg {
        Msg::Turn {
            active: self.active,
            round: self.round,
            wind: self.wind,
            health: self.health(),
            cannon_y: self.cannon_y(),
            winner: match self.phase {
                Phase::Finished(winner) => Some(winner),
                _ => None,
            },
        }
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
                Phase::Aiming
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

#[allow(clippy::too_many_arguments)]
pub fn input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mode: Option<Res<PlayMode>>,
    mut net: Option<ResMut<Net>>,
    mut game: ResMut<Game>,
    mut terrain: ResMut<Terrain>,
    mut meshes: ResMut<Assets<Mesh>>,
    buttons: Query<(&Interaction, &Action), Changed<Interaction>>,
) {
    let online = mode.as_deref() == Some(&PlayMode::Online);
    let waiting = net.as_ref().is_some_and(|net| net.waiting);
    if mode.as_deref() == Some(&PlayMode::Menu) || waiting {
        return;
    }
    let host = net::is_authority(mode.as_deref(), net.as_deref());
    let my_turn = net::my_turn(mode.as_deref(), net.as_deref(), &game);
    let mut primary = keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter);
    let mut restart = keys.just_pressed(KeyCode::KeyR);
    for (interaction, action) in &buttons {
        if *interaction == Interaction::Pressed {
            match action {
                Action::Primary => primary = true,
                Action::Restart => restart = true,
                Action::SelectSize(size) if host => game.next_size = *size,
                Action::SelectSize(_) => {}
            }
        }
    }
    if restart && host {
        let (seed, size) = if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight)
        {
            (game.seed, game.size)
        } else {
            (fresh_seed(), game.next_size)
        };
        Game::restart_world(seed, size, &mut game, &mut terrain, &mut meshes);
        if let Some(net) = net.as_deref() {
            net.send(&Msg::NewMatch { seed, size });
        }
        return;
    }
    if keys.just_pressed(KeyCode::KeyM) {
        game.muted = !game.muted;
    }
    if keys.just_pressed(KeyCode::Escape) && !online {
        game.paused = !game.paused;
    }
    if game.paused || !my_turn {
        return;
    }
    if primary {
        if game.phase == Phase::Handoff {
            if host {
                game.phase = Phase::Aiming;
            }
            if let Some(net) = net.as_deref() {
                net.send(&Msg::Ready);
            }
        } else if host {
            game.fire();
            if game.phase == Phase::Flying
                && let Some(net) = net.as_deref()
            {
                let cannon = game.cannons[game.active];
                net.send(&Msg::ShotFired {
                    yaw: cannon.yaw,
                    elevation: cannon.elevation,
                    power: cannon.power,
                    z: cannon.position.z,
                });
            }
        } else if let Some(net) = net.as_deref() {
            let cannon = game.cannons[game.active];
            net.send(&Msg::Fire {
                yaw: cannon.yaw,
                elevation: cannon.elevation,
                power: cannon.power,
                z: cannon.position.z,
            });
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
    let reach = game.size.lane_reach();
    let along = if active == 0 { 1.0 } else { -1.0 };
    let cannon = &mut game.cannons[active];
    cannon.yaw += axis(KeyCode::KeyD, KeyCode::KeyA) * dt * 0.55;
    cannon.yaw = cannon.yaw.rem_euclid(std::f32::consts::TAU);
    cannon.elevation = (cannon.elevation + axis(KeyCode::KeyW, KeyCode::KeyS) * dt * 0.45)
        .clamp(5.0_f32.to_radians(), 85.0_f32.to_radians());
    cannon.power =
        (cannon.power + axis(KeyCode::KeyE, KeyCode::KeyQ) * dt * 14.0).clamp(15.0, 52.0);
    let dz = along * axis(KeyCode::ArrowRight, KeyCode::ArrowLeft) * dt * 12.0;
    if dz != 0.0 {
        let z = cannon.position.z + dz;
        cannon.place_on_lane(z, reach, &terrain);
    }
    if online && let Some(net) = net.as_deref_mut() {
        net.aim_timer += time.delta_secs();
        if net.aim_timer >= 1.0 / 30.0 {
            net.aim_timer = 0.0;
            net.send(&Msg::Aim {
                yaw: cannon.yaw,
                elevation: cannon.elevation,
                power: cannon.power,
                z: cannon.position.z,
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn simulate(
    time: Res<Time<Fixed>>,
    mode: Option<Res<PlayMode>>,
    net: Option<Res<Net>>,
    mut game: ResMut<Game>,
    mut terrain: ResMut<Terrain>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if game.paused {
        return;
    }
    let dt = time.delta_secs();
    let authority = net::is_authority(mode.as_deref(), net.as_deref());
    if let Phase::Resolving(remaining) = game.phase {
        for cannon in &mut game.cannons {
            let ground = terrain
                .height(cannon.position.x, cannon.position.z)
                .unwrap()
                + 1.2;
            cannon.position.y = (cannon.position.y - 10.0 * dt).max(ground);
        }
        if remaining <= dt {
            if authority {
                game.finish_turn();
                if let Some(net) = net.as_deref()
                    && net.is_host()
                {
                    net.send(&game.turn_msg());
                }
            } else {
                game.phase = Phase::Resolving(0.0);
            }
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
    if !authority {
        if game
            .trail
            .last()
            .is_none_or(|p| p.distance(ball.position) > 0.7)
        {
            game.trail.push(ball.position);
        }
        game.ball = Some(ball);
        return;
    }
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
        if let Some(net) = net.as_deref()
            && net.is_host()
        {
            net.send(&Msg::Impact {
                point,
                damage,
                health: game.health(),
                cannon_y: game.cannon_y(),
                direct: direct.is_some(),
            });
        }
    } else if ball.age > MAX_FLIGHT
        || ball.position.y < -20.0
        || ball.position.x.abs() > game.size.half() + 15.0
        || ball.position.z.abs() > game.size.half() + 15.0
    {
        game.ball = None;
        game.phase = Phase::Resolving(0.8);
        game.message = "Out of bounds. Adjust your aim and power next turn.".into();
        if let Some(net) = net.as_deref()
            && net.is_host()
        {
            net.send(&Msg::Miss {
                cannon_y: game.cannon_y(),
            });
        }
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
            let spawns = app.world().resource::<Terrain>().spawns;
            {
                let mut game = app.world_mut().resource_mut::<Game>();
                for (cannon, spawn) in game.cannons.iter().zip(spawns) {
                    assert!((cannon.position.x - spawn.x).abs() < 0.001);
                    assert!((cannon.position.z - spawn.y).abs() < 0.001);
                }
                assert!((game.cannons[0].position.x + size.lane_reach()).abs() < 0.001);
                assert!((game.cannons[1].position.x - size.lane_reach()).abs() < 0.001);
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
            (MapSize::Large, MapSize::Medium, seed)
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
        assert_eq!(game.phase, Phase::Aiming);
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
        assert_eq!((g.active, g.round, g.phase), (1, 1, Phase::Aiming));
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

    fn input_app(game: Game, terrain: Terrain, dt: f32) -> App {
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs_f32(dt));
        let mut app = App::new();
        app.insert_resource(game)
            .insert_resource(terrain)
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(time)
            .insert_resource(ButtonInput::<KeyCode>::default())
            .add_systems(Update, input);
        app
    }

    #[test]
    fn arrows_slide_the_active_tank_along_its_spawn_lane() {
        let terrain = Terrain::new(1, MapSize::Small);
        let mut game = Game::new(1, &terrain);
        game.phase = Phase::Aiming;
        let start = game.cannons[0].position;
        let reach = MapSize::Small.lane_reach();
        let mut app = input_app(game, terrain, 0.05);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowRight);
        app.update();
        {
            let cannon = app.world().resource::<Game>().cannons[0];
            assert!((cannon.position.x - start.x).abs() < 0.001);
            assert!((cannon.position.z - (start.z + 0.6)).abs() < 0.001);
            assert!(cannon.position.z.abs() <= reach + 0.001);
        }

        app.world_mut().resource_mut::<Game>().cannons[0].position.z = reach;
        app.update();
        assert!((app.world().resource::<Game>().cannons[0].position.z - reach).abs() < 0.001);

        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(KeyCode::ArrowLeft);
        }
        app.world_mut().resource_mut::<Game>().cannons[0].position.z = -reach;
        app.update();
        assert!((app.world().resource::<Game>().cannons[0].position.z + reach).abs() < 0.001);

        app.world_mut().resource_mut::<Game>().phase = Phase::Handoff;
        app.world_mut().resource_mut::<Game>().cannons[0].position.z = 0.0;
        app.update();
        assert_eq!(app.world().resource::<Game>().cannons[0].position.z, 0.0);

        let terrain = Terrain::new(1, MapSize::Small);
        let mut game = Game::new(1, &terrain);
        game.phase = Phase::Aiming;
        game.active = 1;
        let start = game.cannons[1].position;
        let mut app = input_app(game, terrain, 0.05);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowRight);
        app.update();
        let cannon = app.world().resource::<Game>().cannons[1];
        assert!((cannon.position.x - start.x).abs() < 0.001);
        assert!((cannon.position.z - (start.z - 0.6)).abs() < 0.001);
    }
}

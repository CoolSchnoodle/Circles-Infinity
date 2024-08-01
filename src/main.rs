use bevy::{
    prelude::*,
    color::palettes::css::*,
};
use bevy::color::palettes::tailwind::*;
use bevy::input::common_conditions::{input_pressed, input_just_pressed};

use bevy_prototype_lyon::prelude::*;

use rand::random;

// Look... there isn't a good way to draw a circle
// So instead I'm just making a many-sided regular polygon
macro_rules! circle {
    ($radius:expr, $pos:expr) => {{
        let rad_f32 = ::std::convert::Into::<f32>::into($radius);
        let circle = ::bevy_prototype_lyon::shapes::RegularPolygon {
            sides: 8 + (rad_f32 * 0.25).floor() as usize * 3usize,
            feature: ::bevy_prototype_lyon::shapes::RegularPolygonFeature::Radius(rad_f32),
            center: ::std::convert::Into::<Vec2>::into($pos),
        };
        ::bevy_prototype_lyon::entity::ShapeBundle {
            path: ::bevy_prototype_lyon::geometry::GeometryBuilder::build_as(&circle),
            ..default()
        }
    }}
}

fn main() {
    App::new()
        .insert_resource(Msaa::Sample4)
        .add_plugins((DefaultPlugins, ShapePlugin))
        .add_systems(Startup, (
            create_player,
            create_wave_counter,
            create_camera,
            start_on_menu,
        ))
        .add_systems(Update, (
            handle_menu_input.run_if(resource_exists::<MainMenu>),
            handle_paused_input.run_if(resource_exists::<Paused>),
            handle_running_input.run_if(resource_exists::<Running>),
            handle_guide_input.run_if(resource_exists::<ViewingGuide>),

            update_player_color.run_if(resource_exists::<Running>),
            move_projectiles.run_if(resource_exists::<Running>),

            tick_player_timers.run_if(resource_exists::<Running>),
            player_ranged_attack
                .after(tick_player_timers)
                .run_if(|mouse: Res<ButtonInput<MouseButton>>| mouse.pressed(MouseButton::Left) || mouse.just_pressed(MouseButton::Left))
                .run_if(resource_exists::<Running>),

            (spawn_wave_if_no_enemies, destroy_an_enemy, remove_dead_enemies)
                .run_if(resource_exists::<Running>)
                .chain(),
        ))
        .run();
}

fn destroy_an_enemy(mut commands: Commands, query: Query<Entity, With<Enemy>>) {
    commands.entity(
        query.iter().next().expect("Spawned a wave if no enemies right before this")
    ).despawn();
}

const ENEMY_COLOR: Srgba = ORANGE_800;
const ENEMY_RADIUS: f32 = 40.0;

const PLAYER_COLOR_MAX_HP: Srgba = GREEN_800;
const PLAYER_COLOR_NO_HP: Srgba = RED_800;
const PLAYER_RADIUS: f32 = 50.0;

const PROJECTILE_RADIUS: f32 = 15.0;

fn handle_menu_input(mut commands: Commands, keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.pressed(KeyCode::Enter) {
        commands.remove_resource::<MainMenu>();
        commands.insert_resource(Running);
    }
    if keyboard.pressed(KeyCode::KeyG) {
        commands.remove_resource::<MainMenu>();
        commands.insert_resource(ViewingGuide);
    }
}

fn handle_running_input(mut commands: Commands, keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.just_pressed(KeyCode::Escape) {
        commands.remove_resource::<Running>();
        commands.insert_resource(Paused);
    }
}

fn handle_paused_input(mut commands: Commands, keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.just_pressed(KeyCode::Space) {
        commands.remove_resource::<Paused>();
        commands.insert_resource(Running);
    }
    if keyboard.just_pressed(KeyCode::Home) {
        commands.remove_resource::<Paused>();
        commands.insert_resource(MainMenu);
    }
}

fn handle_guide_input(mut commands: Commands, keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.pressed(KeyCode::Escape) {
        commands.remove_resource::<ViewingGuide>();
        commands.insert_resource(MainMenu);
    }
}


fn create_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn create_player(mut commands: Commands) {
    commands.spawn((
        Player,
        PlayerStats::default(),
        PlayerState::from_player_stats(PlayerStats::default()),
        Position::new(0f32, 0f32),
        Health::new(100),
        circle!(PLAYER_RADIUS, Position::new(0.0, 0.0)),
        Fill::color(PLAYER_COLOR_MAX_HP),
        Stroke::new(BLACK, 5.0),
    ));
}

fn start_on_menu(mut commands: Commands) {
    commands.insert_resource(MainMenu);
}

fn create_wave_counter(mut commands: Commands) {
    commands.insert_resource(WaveCounter::default());
}


fn tick_player_timers(time: Res<Time>, mut query: Query<&mut PlayerState, With<Player>>) {
    let mut state = query.single_mut();

    if state.heal_timer.finished() { state.heal_timer.reset() }
    if state.close_attack_timer.finished() { state.close_attack_timer.reset() }
    if state.ranged_attack_timer.finished() { state.ranged_attack_timer.reset() }

    let dt = time.delta();
    state.heal_timer.tick(dt);
    state.close_attack_timer.tick(dt);
    state.ranged_attack_timer.tick(dt);
}

fn player_ranged_attack(
    mut commands: Commands,
    window: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    query: Query<(&PlayerStats, &PlayerState, &Position), With<Player>>
) {
    let (stats, state, pos) = query.single();

    let (camera, transform) = camera.single();
    let Some(relative_mouse_coords): Option<Vec2> = window.single()
        .cursor_position()
        .and_then(|mouse| camera.viewport_to_world_2d(transform, mouse))
        .map(|mouse| mouse - Vec2::new(pos.x, pos.y))
    else { return };

    if !state.ranged_attack_timer.just_finished() {
        return;
    }

    let damage = stats.ranged_attack_damage;
    let velocity = {
        let length = relative_mouse_coords.length();
        let x = relative_mouse_coords.x / length;
        let y = relative_mouse_coords.y / length;
        let scale = stats.ranged_attack_speed;
        Vec2::new(x * scale, y * scale)
    };
    let location = pos.into();
    let pierce_left = stats.ranged_attack_pierce;

    commands.spawn((
        Projectile { damage, velocity, location, pierce_left },
        circle!(PROJECTILE_RADIUS, pos),
        Fill::color(YELLOW_GREEN),
        Stroke::new(BLACK, 5f32),
    ));
}

fn move_projectiles(time: Res<Time>, mut query: Query<(&mut Projectile, &mut Path)>) {
    for (mut proj, mut path) in query.iter_mut() {
        let vel = proj.velocity;
        proj.location += vel * time.delta_seconds();
        *path = circle!(PROJECTILE_RADIUS, proj.location).path;
    }
}

fn update_player_color(mut query: Query<(&mut Fill, &Health), With<Player>>) {

    let (mut fill, health) = match query.iter_mut().next() {
        Some(t) => t,
        None => unreachable!("player should always exist if the game is running")
    };

    let health_percent = (health.max_health() as f32) / (health.current_health() as f32);

    fn mix(min: f32, max: f32, percent: f32) -> f32 {
        return (min * (1.0 - percent)) + (max * percent)
    }

    fill.color = Srgba::rgb(
        mix(PLAYER_COLOR_NO_HP.red, PLAYER_COLOR_MAX_HP.red, health_percent),
        mix(PLAYER_COLOR_NO_HP.green, PLAYER_COLOR_MAX_HP.green, health_percent),
        mix(PLAYER_COLOR_NO_HP.blue, PLAYER_COLOR_MAX_HP.blue, health_percent),
    ).into();
}

fn remove_dead_enemies(mut commands: Commands, query: Query<(Entity, &Health), With<Enemy>>) {
    for enemy in query.iter() {
        if enemy.1.current_health() == 0 {
            commands.entity(enemy.0).despawn();
        }
    }
}

fn spawn_wave_if_no_enemies(
    mut commands: Commands,
    mut wave_counter: ResMut<WaveCounter>,
    query: Query<&Enemy>
) {
    use num_traits::float::FloatConst;

    // If the iterator returns Some(_), then there are still enemies left.
    // Therefore, a new wave should not be spawned.
    if query.iter().next().is_some() {
        return;
    }

    // Since we're spawning a new wave, increment the wave counter.
    // Note: WaveCounter::default() is 0 so the first wave will be 1.
    wave_counter.0 += 1;

    let enemies_to_spawn = 8 + (2 * wave_counter.0);
    const ENEMY_DISTANCE_MIN: f32 = 512f32;
    const ENEMY_DISTANCE_MAX: f32 = 1024f32;
    const ENEMY_DISTANCE_DIFF: f32 = ENEMY_DISTANCE_MAX - ENEMY_DISTANCE_MIN;
    for _ in 0..enemies_to_spawn {
        let angle_radians: f32 = random::<f32>() * f32::PI() * 2.0;
        let distance: f32 = ENEMY_DISTANCE_MIN + (random::<f32>() * ENEMY_DISTANCE_DIFF);
        let pos = Position::new(
            (angle_radians.cos() * distance).into(),
            (angle_radians.sin() * distance).into(),
        );
        commands.spawn((
            Enemy,
            pos,
            Health::new(wave_counter.0 as usize * 2 + 50),
            EnemyStats::new(
                10,
                1f32,
                0,
                1f32,
                100f32,
                48f32 + (wave_counter.0 * 2) as f32,
            ),
            circle!(ENEMY_RADIUS, pos),
            Fill::color(ENEMY_COLOR),
            Stroke::new(BLACK, 3.0),
        ));
    }
}


fn apply_player_upgrade(stats: &mut PlayerStats, health: &mut Health, upgrade: PlayerUpgrade) {
    match upgrade {
        PlayerUpgrade::InstantHeal => health.heal(health.max_health() / 4),
        PlayerUpgrade::HealOverTime => stats.heal_per_second += 2,
        PlayerUpgrade::MaxHp => health.add_max_hp(15),

        PlayerUpgrade::CloseAttackCooldown => stats.close_attack_cooldown *= 0.8,
        PlayerUpgrade::CloseAttackDamage => stats.close_attack_damage += 5,

        PlayerUpgrade::RangedAttackCooldown => stats.ranged_attack_cooldown *= 0.9,
        PlayerUpgrade::RangedAttackDamage => stats.ranged_attack_damage += 3,
        PlayerUpgrade::RangedAttackPierce => stats.ranged_attack_pierce += 2,
        PlayerUpgrade::RangedAttackSpeed => stats.ranged_attack_speed *= 1.25,

        PlayerUpgrade::MoveSpeed => stats.movement_speed *= 1.15,
    }
}

#[derive(Resource)]
#[derive(Debug, Copy, Clone)]
struct Running;
#[derive(Resource)]
#[derive(Debug, Copy, Clone)]
struct Paused;
#[derive(Resource)]
#[derive(Debug, Copy, Clone)]
struct MainMenu;
#[derive(Resource)]
#[derive(Debug, Copy, Clone)]
struct ViewingGuide;

#[derive(Resource)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Default)]
struct WaveCounter(pub u64);

#[derive(Component)]
#[derive(Debug, Copy, Clone, Default)]
struct Player;

#[derive(Component)]
#[derive(Debug, Copy, Clone, Default)]
struct Enemy;

#[derive(Component)]
#[derive(Debug, Copy, Clone, PartialEq)]
struct Projectile {
    pub damage: usize,
    pub velocity: Vec2,
    pub location: Vec2,
    pub pierce_left: usize,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum PlayerUpgrade {
    InstantHeal,
    HealOverTime,
    MaxHp,

    CloseAttackDamage,
    CloseAttackCooldown,

    RangedAttackDamage,
    RangedAttackCooldown,
    RangedAttackPierce,
    RangedAttackSpeed,

    MoveSpeed,
}

#[derive(Component)]
#[derive(Debug, Copy, Clone, PartialEq, Default)]
struct Position { pub x: f32, pub y: f32 }

#[derive(Component)]
#[derive(Debug, Copy, Clone, PartialEq)]
struct PlayerStats {
    close_attack_damage: usize,
    close_attack_cooldown: f32,

    ranged_attack_damage: usize,
    ranged_attack_cooldown: f32,
    ranged_attack_pierce: usize,
    ranged_attack_speed: f32,

    movement_speed: f32,
    heal_per_second: usize,
}

#[derive(Component)]
#[derive(Debug, Clone)]
struct PlayerState {
    close_attack_timer: Timer,
    ranged_attack_timer: Timer,
    heal_timer: Timer,
    facing_radians: f32,
}

#[derive(Component)]
#[derive(Debug, Copy, Clone, PartialEq)]
struct EnemyStats {
    close_attack_damage: usize,
    close_attack_cooldown: f32,

    ranged_attack_damage: usize,
    ranged_attack_cooldown: f32,
    ranged_attack_speed: f32,

    movement_speed: f32,
}

#[derive(Component)]
#[derive(Debug, Clone)]
struct EnemyState {
    close_attack_timer: Timer,
    ranged_attack_timer: Timer,
}

#[derive(Component)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
struct Health {
    max_hp: usize,
    current_hp: usize
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn x_pos(&self) -> f32 {
        self.x
    }
    pub fn y_pos(&self) -> f32 {
        self.y
    }
    pub fn set_x(&mut self, new_x: f32) {
        self.x = new_x;
    }
    pub fn set_y(&mut self, new_y: f32) {
        self.y = new_y;
    }
    pub fn swap_x(&mut self, mut new_x: f32) -> f32 {
        std::mem::swap(&mut self.x, &mut new_x);
        new_x
    }
    pub fn swap_y(&mut self, mut new_y: f32) -> f32 {
        std::mem::swap(&mut self.y, &mut new_y);
        new_y
    }

    fn distance(&self, other: &Self) -> f32 {
        let this: Vec2 = self.into();
        let other: Vec2 = other.into();
        this.distance(other)
    }
}

impl PlayerState {
    fn from_player_stats(player_stats: PlayerStats) -> Self {
        Self {
            close_attack_timer: Timer::from_seconds(
                player_stats.close_attack_cooldown,
                TimerMode::Repeating,
            ),
            ranged_attack_timer: Timer::from_seconds(
                player_stats.ranged_attack_cooldown,
                TimerMode::Repeating,
            ),
            heal_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            facing_radians: 0f32,
        }
    }
}

impl EnemyStats {
    fn new(
        close_attack_damage: usize,
        close_attack_cooldown: f32,

        ranged_attack_damage: usize,
        ranged_attack_cooldown: f32,
        ranged_attack_speed: f32,

        movement_speed: f32,
    ) -> Self {
        Self {
            close_attack_damage,
            close_attack_cooldown,
            ranged_attack_damage,
            ranged_attack_cooldown,
            ranged_attack_speed,
            movement_speed,
        }
    }
}

impl EnemyState {
    fn from_enemy_stats(enemy_stats: EnemyStats) -> Self {
        Self {
            close_attack_timer:
                Timer::from_seconds(enemy_stats.close_attack_cooldown, TimerMode::Repeating),
            ranged_attack_timer:
                Timer::from_seconds(enemy_stats.ranged_attack_cooldown, TimerMode::Repeating),
        }
    }
}

impl Health {
    pub fn new(max_hp: usize) -> Self {
        Self { max_hp, current_hp: max_hp }
    }
    pub fn with_max_and_current(max_hp: usize, current_hp: usize) -> Option<Self> {
        if current_hp > max_hp { None }
        else { Some( Self{ max_hp, current_hp } ) }
    }
    pub fn max_health(&self) -> usize {
        self.max_hp
    }
    pub fn current_health(&self) -> usize {
        self.current_hp
    }

    /// Heals an amount of health.
    /// This function caps healing to the maximum health.
    /// If you want to be able to heal over the maximum health,
    /// use `Health::add_health()`.
    pub fn heal(&mut self, heal_amount: usize) {
        self.current_hp = self.max_hp.max(self.current_hp + heal_amount)
    }

    /// Heals an amount of health.
    /// This function will not cap healing to the maximum health.
    /// If you want healing to be capped at the maximum health,
    /// use `Health::heal()`
    pub fn add_health(&mut self, heal_amount: usize) {
        self.current_hp += heal_amount
    }

    /// Deal an amount of damage.
    /// This function returns whether that amount of damage reduced
    /// the current health total to 0.
    ///
    /// If the amount of damage is greater than or equal to the current health,
    /// the current health is reduced to 0 and `true` is returned.
    ///
    /// If the amount of damage is less than the current health,
    /// the current health is reduced by the amount of damage
    /// and `false` is returned.
    pub fn damage(&mut self, damage_amount: usize) -> bool {
        if damage_amount >= self.current_hp {
            self.current_hp = 0;
            true
        } else {
            self.current_hp -= damage_amount;
            false
        }
    }

    pub fn add_max_hp(&mut self, extra_max_hp: usize) {
        self.max_hp += extra_max_hp;
    }

    /// Reduces maximum health.
    /// This function returns whether the maximum health loss
    /// brought the maximum health to zero.
    /// This function will reduce the current health if
    /// it is greater than the new maximum health value.
    /// To instead not reduce the current health if it
    /// is greater than the maximum health after the
    /// maximum health reduction, use `Health::remove_max_hp`
    pub fn reduce_max_hp(&mut self, max_hp_loss: usize) -> bool {
        if max_hp_loss >= self.max_hp {
            self.max_hp = 0;
            self.current_hp = 0;
            true
        } else {
            self.max_hp -= max_hp_loss;
            self.current_hp = self.current_hp.max(self.max_hp);
            false
        }
    }

    /// Removes maximum health.
    /// This function returns whether the maximum health loss
    /// brought the maximum health to zero.
    /// This function will not reduce the current health, even if
    /// it is greater than the new maximum health value.
    /// To instead reduce the current health if it is greater
    /// than the maximum health after the maximum health reduction,
    /// use `Health::reduce_max_hp()`
    pub fn remove_max_hp(&mut self, max_hp_loss: usize) -> bool {
        if max_hp_loss >= self.max_hp {
            self.max_hp = 0;
            true
        } else {
            self.max_hp -= max_hp_loss;
            false
        }
    }

    pub fn set_max_hp(&mut self, new_max_hp: usize) {
        self.max_hp = new_max_hp;
    }
    pub fn set_current_hp(&mut self, new_hp: usize) {
        self.current_hp = new_hp;
    }
    pub fn swap_max_hp(&mut self, mut new_max_hp: usize) -> usize {
        std::mem::swap(&mut self.max_hp, &mut new_max_hp);
        new_max_hp
    }
    pub fn swap_current_hp(&mut self, mut new_hp: usize) -> usize {
        std::mem::swap(&mut self.max_hp, &mut new_hp);
        new_hp
    }
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            close_attack_damage: 40usize,
            close_attack_cooldown: 1f32,

            ranged_attack_damage: 5usize,
            ranged_attack_cooldown: 0.15f32,
            ranged_attack_pierce: 4usize,
            ranged_attack_speed: 250f32,

            movement_speed: 30f32,
            heal_per_second: 5usize,
        }
    }
}

impl std::ops::Add for Position {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}
impl std::ops::Sub for Position {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}
impl std::ops::Mul<f32> for Position {
    type Output = Self;
    fn mul(self, multiplier: f32) -> Self::Output {
        Self {
            x: self.x * multiplier,
            y: self.y * multiplier,
        }
    }
}

impl Into<Vec2> for Position {
    fn into(self) -> Vec2 {
        Vec2 {
            x: self.x,
            y: self.y
        }
    }
}
impl Into<Vec2> for &Position {
    fn into(self) -> Vec2 {
        (*self).into()
    }
}

impl From<Vec2> for Position {
    fn from(value: Vec2) -> Self {
        Self::new(value.x, value.y)
    }
}
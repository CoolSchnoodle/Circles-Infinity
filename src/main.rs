use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (
            create_player,
            create_wave_counter,
        ))
        .add_systems(Update, (
            remove_dead_enemies,
        ))
        .run();
}

fn create_player(mut commands: Commands) {
    commands.spawn((
        Player,
        PlayerStats::default(),
        PlayerState::from_player_stats(PlayerStats::default()),
        Position::new(0f32, 0f32),
        Health::new(100),
    ));
}

fn create_wave_counter(mut commands: Commands) {
    commands.insert_resource(WaveCounter::default());
}

fn remove_dead_enemies(mut commands: Commands, query: Query<(Entity, &Health), With<Enemy>>) {
    for enemy in query.iter() {
        if enemy.1.current_health() == 0 {
            commands.entity(enemy.0).despawn();
        }
    }
}

#[derive(Resource)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Default)]
struct WaveCounter(pub u64);

#[derive(Resource)]
#[derive(Debug, Clone, Default)]
struct WaveSpawnTimer(pub Timer);

#[derive(Component)]
#[derive(Debug, Copy, Clone, Default)]
struct Player;

#[derive(Component)]
#[derive(Debug, Copy, Clone, Default)]
struct Enemy;

#[derive(Component)]
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Default)]
struct Position { x: f32, y: f32 }

#[derive(Component)]
#[derive(Debug, Copy, Clone, PartialEq)]
struct PlayerStats {
    close_attack_damage: usize,
    close_attack_cooldown: f32,

    ranged_attack_damage: usize,
    ranged_attack_cooldown: f32,
    ranged_attack_seeking: i8,
    ranged_attack_speed: f32,

    movement_speed: f32,
}

#[derive(Component)]
#[derive(Debug, Clone)]
struct PlayerState {
    close_attack_timer: Timer,
    ranged_attack_timer: Timer,
    facing: u8,
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
            facing: 0u8,
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
            close_attack_damage: 30usize,
            close_attack_cooldown: 1f32,

            ranged_attack_damage: 5usize,
            ranged_attack_cooldown: 0.5f32,
            ranged_attack_seeking: 0i8,
            ranged_attack_speed: 20f32,

            movement_speed: 30f32
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
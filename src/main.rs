use bevy::{
    prelude::*,
    color::palettes::css::*,
};
use bevy::color::palettes::tailwind::*;
use bevy::input::common_conditions::input_pressed;
use bevy::render::view::NoFrustumCulling;
use bevy_prototype_lyon::prelude::*;

use rand::random;

// Look... there isn't a good way to draw a circle
// So instead I'm just making a many-sided regular polygon
macro_rules! circle {
    ($radius:expr, $pos:expr) => {{
        let rad_f32 = ::std::convert::Into::<f32>::into($radius);
        let circle = ::bevy_prototype_lyon::shapes::RegularPolygon {
            sides: 6 + (rad_f32 * 0.2).floor() as usize * 3usize,
            feature: ::bevy_prototype_lyon::shapes::RegularPolygonFeature::Radius(rad_f32),
            center: ::std::convert::Into::<Vec2>::into($pos),
        };
        ::bevy_prototype_lyon::entity::ShapeBundle {
            path: ::bevy_prototype_lyon::geometry::GeometryBuilder::build_as(&circle),
            ..default()
        }
    }}
}

macro_rules! to_vec2 {
    ($pos:expr) => {
        ::std::convert::Into::<Vec2>::into($pos)
    }
}

fn main() {
    App::new()
        .insert_resource(Msaa::Sample4)
        .add_plugins((
            DefaultPlugins,
            ShapePlugin,
        ))
        .add_systems(Startup, (
            setup,
            start_on_menu,
        ).chain())
        .add_systems(Update, (
            guide_to_menu.run_if(resource_exists::<Transition<Guide, MainMenu>>),
            menu_to_guide.run_if(resource_exists::<Transition<MainMenu, Guide>>),
            menu_to_running.run_if(resource_exists::<Transition<MainMenu, Running>>),
            running_to_lose_screen
                .run_if(resource_exists::<Transition<Running, LoseScreen>>)
                .before(check_and_resolve_player_death),
            lose_screen_to_running.run_if(resource_exists::<Transition<LoseScreen, Running>>),
            lose_screen_to_menu.run_if(resource_exists::<Transition<LoseScreen, MainMenu>>),

            handle_menu_input.run_if(resource_exists::<MainMenu>),
            handle_paused_input.run_if(resource_exists::<Paused>),
            handle_running_input.run_if(resource_exists::<Running>),
            handle_guide_input.run_if(resource_exists::<Guide>),
            handle_lose_screen_input.run_if(resource_exists::<LoseScreen>),

            update_player_color.run_if(resource_exists::<Running>),
            update_player_health_text.run_if(resource_exists::<Running>),

            update_player
                .before(player_ranged_attack)
                .before(enemy_update_and_attack)
                .run_if(resource_exists::<Running>),

            (
                update_power_ups,
                create_power_ups,
                collect_power_ups
            ).run_if(resource_exists::<Running>).chain(),
            (
                player_ranged_attack
                    .run_if(input_pressed(MouseButton::Left)),
                resolve_enemy_projectiles,
                check_and_resolve_player_death,
            ).run_if(resource_exists::<Running>).chain(),

            resolve_player_projectiles.run_if(resource_exists::<Running>),

            (
                move_projectiles,
                do_splashes,
                handle_residue,
                remove_dead_enemies,
                enemy_update_and_attack,
                spawn_wave_if_no_enemies,
            ).run_if(resource_exists::<Running>).chain()

        ))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.insert_resource(WaveCounter::default());
    commands.insert_resource(PlayerUpgradeCounter::default());
    commands.insert_resource(HighScore::default());
    commands.insert_resource(PowerUpSpawnTimer::default())
}

const ENEMY_COLOR: Srgba = ORANGE_800;
const ENEMY_RADIUS: f32 = 40.0;

const PLAYER_COLOR_MAX_HP: Srgba = GREEN_800;
const PLAYER_COLOR_NO_HP: Srgba = RED_800;
const PLAYER_RADIUS: f32 = 50.0;

const BASE_PROJECTILE_RADIUS: f32 = 15.0;

fn menu_to_running(
    mut player_upgrade_counter: ResMut<PlayerUpgradeCounter>,
    mut wave_counter: ResMut<WaveCounter>,
    mut commands: Commands,
    main_menu_items: Query<Entity, With<MainMenuItem>>
) {
    for id in main_menu_items.iter() {
        commands.entity(id).despawn()
    }

    let stats = PlayerStats::default();
    commands.spawn((
        RunningObject,
        NoFrustumCulling, // prevent weird invisibility
        Player,
        Health::new(100),
        stats,
        PlayerState::from_player_stats(stats),
        Position::new(0.0, 0.0),
        circle!(PLAYER_RADIUS, Position::new(0.0, 0.0)),
        Fill::color(PLAYER_COLOR_MAX_HP),
        Stroke::new(BLACK, 5.0),
    ));

    wave_counter.0 = 0;
    commands.spawn((
        RunningObject,
        WaveCounterText,
        TextBundle::from_section("Wave 0", TextStyle::default())
            .with_style(Style {
                position_type: PositionType::Absolute,
                top: Val::Percent(2.0),
                left: Val::Percent(2.0),
                ..default()
            }),
    ));

    player_upgrade_counter.reset();
    commands.spawn((
        RunningObject,
        PlayerUpgradeCounterText,
        TextBundle::from_section(player_upgrade_counter.display_text(), TextStyle::default())
            .with_style(Style {
                position_type: PositionType::Absolute,
                bottom: Val::Percent(2.0),
                left: Val::Percent(2.0),
                ..default()
            }),
    ));

    commands.spawn((
        RunningObject,
        PlayerHealthText,
        TextBundle::from_section("Health: 100 / 100", TextStyle::default())
            .with_style(Style {
                position_type: PositionType::Absolute,
                top: Val::Percent(5.0),
                left: Val::Percent(2.0),
                ..default()
            })
    ));

    commands.remove_resource::<Transition<MainMenu, Running>>();
    commands.insert_resource(Running);
}

fn menu_to_guide(mut commands: Commands, entities: Query<Entity, With<MainMenuItem>>) {
    const GUIDE_TEXT: &'static str = "Welcome to the guide, where you learn how the game works.\n\n\
    You are the green circle (although the color will become more red as you lose health). Enemies, \
    which are orange circles will spawn around you in waves. Your goal is to survive as many waves \
    as possible. To get to the next wave, you will need to kill every enemy. By left clicking and \
    holding, you will create projectiles which damage enemies. You can press Q or E to switch \
    which projectile you are using.\n\n\
    You can also upgrade your player by pressing one of the number keys (you get an extra upgrade \
    after beating each round). The types of upgrades are Health, Attack, and Speed.\n\n\
    Brightly colored, pulsating powerups will periodically spawn. Collecting these will give you \
    temporary boosts!\n\n\
    Press Escape to return to the home screen.";

    for id in entities.iter() {
        commands.entity(id).despawn();
    }

    commands.spawn((
        GuideItem,
        TextBundle::from_section(GUIDE_TEXT, TextStyle::default())
            .with_style(Style {
                position_type: PositionType::Absolute,
                top: Val::Percent(25.0),
                left: Val::Percent(10.0),
                right: Val::Percent(10.0),
                ..default()
            })
    ));

    commands.remove_resource::<Transition<MainMenu, Guide>>();
    commands.insert_resource(Guide);
}

fn guide_to_menu(mut commands: Commands, entities: Query<Entity, With<GuideItem>>, high_score: Res<HighScore>) {
    for id in entities.iter() {
        commands.entity(id).despawn();
    }

    commands.remove_resource::<Transition<Guide, MainMenu>>();
    start_on_menu(commands, high_score);
}

fn lose_screen_to_running(
    mut player_upgrade_counter: ResMut<PlayerUpgradeCounter>,
    mut wave_counter: ResMut<WaveCounter>,
    mut commands: Commands,
    entities: Query<Entity, With<LoseScreenItem>>,
) {
    for entity in entities.iter() {
        commands.entity(entity).despawn();
    }

    let stats = PlayerStats::default();
    commands.spawn((
        RunningObject,
        NoFrustumCulling, // prevent weird invisibility
        Player,
        Health::new(100),
        stats,
        PlayerState::from_player_stats(stats),
        Position::new(0.0, 0.0),
        circle!(PLAYER_RADIUS, Position::new(0.0, 0.0)),
        Fill::color(PLAYER_COLOR_MAX_HP),
        Stroke::new(BLACK, 5.0),
    ));

    wave_counter.0 = 0;
    commands.spawn((
        RunningObject,
        WaveCounterText,
        TextBundle::from_section("Wave 0", TextStyle::default())
            .with_style(Style {
                position_type: PositionType::Absolute,
                top: Val::Percent(2.0),
                left: Val::Percent(2.0),
                ..default()
            }),
    ));

    player_upgrade_counter.reset();
    commands.spawn((
        RunningObject,
        PlayerUpgradeCounterText,
        TextBundle::from_section(player_upgrade_counter.display_text(), TextStyle::default())
            .with_style(Style {
                position_type: PositionType::Absolute,
                bottom: Val::Percent(2.0),
                left: Val::Percent(2.0),
                ..default()
            }),
    ));

    commands.spawn((
        RunningObject,
        PlayerHealthText,
        TextBundle::from_section("Health: 100 / 100", TextStyle::default())
            .with_style(Style {
                position_type: PositionType::Absolute,
                top: Val::Percent(5.0),
                left: Val::Percent(2.0),
                ..default()
            })
    ));

    commands.remove_resource::<Transition<LoseScreen, Running>>();
    commands.insert_resource(Running);
}

fn lose_screen_to_menu(mut commands: Commands, entities: Query<Entity, With<LoseScreenItem>>, high_score: Res<HighScore>) {
    for entity in entities.iter() {
        commands.entity(entity).despawn();
    }

    commands.remove_resource::<Transition<LoseScreen, MainMenu>>();
    start_on_menu(commands, high_score);
}

fn running_to_lose_screen(
    wave_counter: Res<WaveCounter>,
    mut high_score: ResMut<HighScore>,
    mut commands: Commands,
    entities: Query<Entity, With<RunningObject>>,
) {
    for entity in entities.iter() {
        commands.entity(entity).despawn();
    }

    if wave_counter.0 <= high_score.0 {
        commands.spawn((
            LoseScreenItem,
            TextBundle::from_section(format!(
                    "You lost on wave {}, your best wave is {}.\n\n\
                    Thanks for playing! Press enter to play again, \
                    or press Escape to return to the main menu.",
                    wave_counter.0,
                    high_score.0,
                ), TextStyle::default())
                .with_style(Style {
                    position_type: PositionType::Absolute,
                    align_self: AlignSelf::Center,
                    align_content: AlignContent::Center,
                    top: Val::Percent(25.0),
                    bottom: Val::Percent(10.0),
                    left: Val::Percent(10.0),
                    right: Val::Percent(10.0),
                    ..default()
                }),
        ));
    } else {
        commands.spawn((
            LoseScreenItem,
            TextBundle::from_section(format!(
                "New high score: {}! Your previous best wave was {}.\n\n\
                Thanks for playing! Press enter to play again, \
                or press Escape to return to the main menu.",
                wave_counter.0,
                high_score.0,
            ), TextStyle::default())
                .with_style(Style {
                    position_type: PositionType::Absolute,
                    align_self: AlignSelf::Center,
                    align_content: AlignContent::Center,
                    top: Val::Percent(25.0),
                    bottom: Val::Percent(10.0),
                    left: Val::Percent(10.0),
                    right: Val::Percent(10.0),
                    ..default()
                }),
        ));
        high_score.0 = wave_counter.0;
    }

    commands.remove_resource::<Transition<Running, LoseScreen>>();
    commands.insert_resource(LoseScreen);
}

fn handle_menu_input(mut commands: Commands, keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.pressed(KeyCode::Enter) {
        commands.remove_resource::<MainMenu>();
        commands.insert_resource(Transition::new(MainMenu, Running));
    }
    if keyboard.pressed(KeyCode::KeyG) {
        commands.remove_resource::<MainMenu>();
        commands.insert_resource(Transition::new(MainMenu, Guide));
    }
}

fn handle_running_input(
    mut player_upgrade_counter: ResMut<PlayerUpgradeCounter>,
    mut player_upgrade_counter_text: Query<&mut Text, With<PlayerUpgradeCounterText>>,
    mut player: Query<(&mut PlayerStats, &mut Health, &mut PlayerState), With<Player>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // didn't feel like implementing pause menu
    /*
    if keyboard.just_pressed(KeyCode::Escape) {
        commands.remove_resource::<Running>();
        commands.insert_resource(Transition::new(Running, Paused));
    }
    */

    let (mut stats, mut health, mut state) = player.single_mut();

    if keyboard.just_pressed(KeyCode::KeyQ) {
        state.current_weapon.cycle_left();
        state.ranged_attack_timer = Timer::from_seconds(
            stats.ranged_attack_cooldown * state.current_weapon.cooldown_multiplier(),
            TimerMode::Repeating,
        );
    } else if keyboard.just_pressed(KeyCode::KeyE) {
        state.current_weapon.cycle_right();
        state.ranged_attack_timer = Timer::from_seconds(
            stats.ranged_attack_cooldown * state.current_weapon.cooldown_multiplier(),
            TimerMode::Repeating,
        );
    }

    if player_upgrade_counter.unused_upgrades > 0 {
        let upgrade: Option<PlayerUpgrade>;

        if keyboard.just_pressed(KeyCode::Digit1) {
            upgrade = Some(PlayerUpgrade::HealthUpgrade);
        } else if keyboard.just_pressed(KeyCode::Digit2) {
            upgrade = Some(PlayerUpgrade::AttackUpgrade);
        } else if keyboard.just_pressed(KeyCode::Digit3) {
            upgrade = Some(PlayerUpgrade::SpeedUpgrade);
        } else {
            upgrade = None;
        }

        if let Some(upgrade) = upgrade {
            player_upgrade_counter.unused_upgrades -= 1;
            player_upgrade_counter.add_upgrade(upgrade);

            *player_upgrade_counter_text.single_mut() =
                Text::from_section(player_upgrade_counter.display_text(), TextStyle::default());

            apply_player_upgrade(stats.into_inner(), health.into_inner(), upgrade);
        }
    }
}

fn handle_paused_input(mut commands: Commands, keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.just_pressed(KeyCode::Space) {
        commands.remove_resource::<Paused>();
        commands.insert_resource(Transition::new(Paused, Running));
    }
    if keyboard.just_pressed(KeyCode::Home) {
        commands.remove_resource::<Paused>();
        commands.insert_resource(Transition::new(Paused, MainMenu));
    }
}

fn handle_guide_input(mut commands: Commands, keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.pressed(KeyCode::Escape) {
        commands.remove_resource::<Guide>();
        commands.insert_resource(Transition::new(Guide, MainMenu));
    }
}

fn handle_lose_screen_input(mut commands: Commands, keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.pressed(KeyCode::Enter) {
        commands.remove_resource::<LoseScreen>();
        commands.insert_resource(Transition::new(LoseScreen, Running))
    }

    if keyboard.pressed(KeyCode::Escape) {
        commands.remove_resource::<LoseScreen>();
        commands.insert_resource(Transition::new(LoseScreen, MainMenu));
    }
}

fn start_on_menu(mut commands: Commands, high_score: Res<HighScore>) {
    commands.insert_resource(MainMenu);
    commands.spawn((
        MainMenuItem,
        TextBundle::from_section(format!(
            "Welcome to Circles Infinity!\n\n\
            In this game your goal is to survive endless waves of enemies for as long as possible.\n\n\
            To learn how to play, press the G key to view a guide.\n\n\
            If you know how to play, you can press the Enter/Return key to jump right into a game.\n\n\
            The highest wave you've reached is {}.",
            high_score.0
        ), TextStyle::default())
            .with_style(Style {
                position_type: PositionType::Absolute,
                align_self: AlignSelf::Center,
                align_content: AlignContent::Center,
                top: Val::Percent(25.0),
                bottom: Val::Percent(10.0),
                left: Val::Percent(10.0),
                right: Val::Percent(10.0),
                ..default()
            }),
    ));
}

fn remove_dead_enemies(mut commands: Commands, mut query: Query<(Entity, &Health), With<Enemy>>) {
    for (id, hp) in query.iter() {
        if hp.current_health() == 0 {
            commands.entity(id).despawn();
        }
    }
}

fn update_player_health_text(mut player_health_text: Query<&mut Text, With<PlayerHealthText>>, player_health: Query<&Health, With<Player>>) {
    let player_health = player_health.single();
    *player_health_text.single_mut() = Text::from_section(
        format!("Health: {}/{}", player_health.current_hp, player_health.max_hp),
        TextStyle::default(),
    );
}

fn update_player(
    time: Res<Time>,
    mut camera_transform: Query<&mut Transform, With<Camera>>,
    mut player: Query<(&mut Position, &mut Path, &PlayerStats, &mut PlayerState), With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    // update position
    let w = keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp);
    let a = keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft);
    let s = keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown);
    let d = keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight);

    let mut vertical_movement = if w && !s { 1f32 } else if s && !w { -1f32 } else { 0f32 };
    let mut horizontal_movement = if d && !a { 1f32 } else if a && !d { -1f32 } else { 0f32 };
    if vertical_movement != 0.0 && horizontal_movement != 0.0 {
        let sqrt_2 = 2f32.sqrt();
        vertical_movement /= sqrt_2;
        horizontal_movement /= sqrt_2;
    }

    let (mut pos, mut path, stats, mut state) = player.single_mut();
    let mut transform = camera_transform.single_mut();
    let dt = time.delta_seconds();

    if state.boost_time_left > 0.0 {
        state.boost_time_left -= time.delta_seconds();
        state.boost_time_left = f32::max(state.boost_time_left, 0.0);
    }

    let speed_multiplier: f32 = dt * if state.boost_time_left > 0.0 { 2.0 } else { 1.0 };
    pos.y += vertical_movement * speed_multiplier * stats.movement_speed;
    pos.x += horizontal_movement * speed_multiplier * stats.movement_speed;

    *path = circle!(PLAYER_RADIUS, *pos).path;
    transform.translation = transform.translation.lerp(Vec3::new(pos.x, pos.y, transform.translation.z), dt * 3.0);

    // update timers
    if state.close_attack_timer.finished() { state.close_attack_timer.reset() }
    if state.ranged_attack_timer.finished() { state.ranged_attack_timer.reset() }

    let dt = time.delta();
    state.close_attack_timer.tick(dt);
    state.ranged_attack_timer.tick(dt);
}

fn do_splashes(
    mut commands: Commands,
    mut enemies: Query<(&mut Health, &Position), With<Enemy>>,
    splashes: Query<(Entity, &SplashProjectileResidual, &Position)>,
) {
    splashes.iter().for_each(|(id, residual, splash_pos)| {
        enemies.iter_mut().for_each(|(mut enemy_hp, enemy_pos)| {
            if to_vec2!(splash_pos).distance(to_vec2!(enemy_pos)) < ENEMY_RADIUS + residual.radius {
                enemy_hp.damage(residual.damage);
            }
        });
        commands.spawn((
            RunningObject,
            SplashResidue(Timer::from_seconds(0.5, TimerMode::Once)),
            circle!(residual.radius * 0.6, splash_pos),
            Fill::color(PURPLE.with_alpha(0.3)),
        ));
        commands.entity(id).despawn();
    })
}

fn handle_residue(time: Res<Time>, mut commands: Commands, mut residue: Query<(Entity, &mut SplashResidue)>) {
    let dt = time.delta();
    for (id, mut residue) in residue.iter_mut() {
        residue.0.tick(dt);
        if residue.0.finished() {
            commands.entity(id).despawn();
        }
    }
}

fn enemy_update_and_attack(
    time: Res<Time>,
    mut commands: Commands,
    mut player: Query<(&Position, &mut Health), (With<Player>, Without<Enemy>)>,
    mut query: Query<(Entity, &EnemyStats, &mut EnemyState, &mut Position, &mut Path), (With<Enemy>, Without<Player>)>,
) {
    let (player_pos, mut player_hp) = player.single_mut();
    let dt = time.delta();
    for (id, stats, mut state, mut pos, mut path) in query.iter_mut() {

        if state.close_attack_timer.finished() && pos.distance(player_pos) <= ENEMY_RADIUS + PLAYER_RADIUS {
            player_hp.damage(stats.close_attack_damage);
            state.close_attack_timer.reset();
        } else { state.close_attack_timer.tick(dt); }

        let base_movement = (to_vec2!((pos.x, pos.y)) - to_vec2!(player_pos)).normalize_or_zero();
        let move_vector = -stats.movement_speed * base_movement * time.delta_seconds();
        *pos = (to_vec2!((pos.x, pos.y)) + (move_vector)).into();
        *path = circle!(ENEMY_RADIUS, *pos).path;

        if stats.ranged_attack_damage > 0 && state.ranged_attack_timer.finished() {
            commands.spawn((
                RunningObject,
                EnemyProjectile,
                Projectile {
                    damage: stats.ranged_attack_damage,
                    velocity: -base_movement * stats.ranged_attack_speed,
                    location: to_vec2!(*pos),
                    radius: BASE_PROJECTILE_RADIUS,
                    pierce_left: 1,
                    last_entity_hit: id,
                },
                circle!(BASE_PROJECTILE_RADIUS, *pos),
                Fill::color(ORANGE_RED),
                Stroke::new(BLACK, 5f32),
            ));
            state.ranged_attack_timer.reset();
        } else { state.ranged_attack_timer.tick(dt); }
    }
}

fn resolve_player_projectiles(
    mut commands: Commands,
    mut enemies: Query<(Entity, &Position, &mut Health), With<Enemy>>,
    mut query: Query<(Entity, &mut Projectile, Option<&SplashProjectile>), With<PlayerProjectile>>,
) {

    let mut enemies = enemies.iter_mut()
        .map(|(id, loc, hp)| (id, to_vec2!((loc.x, loc.y)), hp))
        .collect::<Vec<_>>();

    for (id, mut projectile, splash_stats) in query.iter_mut() {
        let approx_x = projectile.location.x as isize;
        let approx_y = projectile.location.y as isize;
        let collide_distance: isize = ENEMY_RADIUS as isize + projectile.radius as isize;

        for (enemy_id, enemy_loc, ref mut enemy_health) in enemies.iter_mut() {
            if *enemy_id == projectile.last_entity_hit {
                continue;
            }

            let approx_enemy_x = enemy_loc.x as isize;
            let approx_enemy_y = enemy_loc.y as isize;

            if (approx_enemy_x - approx_x).abs() > collide_distance
            || (approx_enemy_y - approx_y).abs() > collide_distance {
                continue;
            }

            if projectile.location.distance(*enemy_loc) as isize <= collide_distance {
                enemy_health.damage(projectile.damage);
                if let Some(splash_stats) = splash_stats {
                    commands.spawn((
                        RunningObject,
                        SplashProjectileResidual {
                            damage: splash_stats.damage,
                            radius: splash_stats.range,
                        },
                        Position::new(projectile.location.x, projectile.location.y),
                    ));
                }

                match projectile.pierce_left {
                    0 => commands.entity(id).despawn(),
                    _ => projectile.pierce_left -= 1,
                }
                projectile.last_entity_hit = *enemy_id;
            }
        }
    }
}

fn resolve_enemy_projectiles(
    mut commands: Commands,
    mut player: Query<(&Position, &mut Health), With<Player>>,
    mut query: Query<(Entity, &mut Projectile), With<EnemyProjectile>>,
) {
    let mut player = player.single_mut();
    let (player_loc, mut player_hp) = (to_vec2!(player.0), player.1);

    for (id, mut projectile) in query.iter_mut() {
        let approx_x = projectile.location.x as isize;
        let approx_y = projectile.location.y as isize;

        if (approx_x - player_loc.x as isize).abs() > 3000
            || (approx_y - player_loc.y as isize).abs() > 3000 {
            commands.entity(id).despawn();
            continue;
        }

        const COLLIDE_DISTANCE: isize = PLAYER_RADIUS as isize + BASE_PROJECTILE_RADIUS as isize;

        if COLLIDE_DISTANCE > projectile.location.distance(player_loc) as isize {
            player_hp.damage(projectile.damage);
            commands.entity(id).despawn();
        }
    }
}

fn create_power_ups(
    mut commands: Commands,
    mut power_up_spawn_timer: ResMut<PowerUpSpawnTimer>,
    time: Res<Time>,
    player_pos: Query<&Position, With<Player>>
) {
    use num_traits::float::FloatConst;

    const PROJECTILE_DIST_MIN: f32 = 300f32;
    const PROJECTILE_DIST_MAX: f32 = 800f32;
    let projectile_dist_diff = PROJECTILE_DIST_MAX - PROJECTILE_DIST_MIN;

    power_up_spawn_timer.0.tick(time.delta());
    if !power_up_spawn_timer.0.just_finished() {
        return;
    }
    power_up_spawn_timer.0.reset();

    let player_pos = player_pos.single();
    let distance = (random::<f32>() * projectile_dist_diff) + PROJECTILE_DIST_MIN;
    let angle_radians = random::<f32>() * f32::PI() * 2.0;
    let pos = Position::new(
        player_pos.x + (angle_radians.cos() * distance),
        player_pos.y + (angle_radians.sin() * distance),
    );

    let power_up_type = PowerUpType::random();

    commands.spawn((
        RunningObject,
        PowerUp { time_since_created: 0.0 },
        power_up_type,
        pos,
        circle!(15.0, pos),
        Fill::color(power_up_type.color()),
        Stroke::new(power_up_type.color().with_luminance(0.3), 2.0),
    ));
}

fn power_up_radius(time_since_created: f32) -> f32 {
    use std::ops::Mul;
    time_since_created.mul(5.0).sin().mul(3.0) + 15.0
}

fn update_power_ups(time: Res<Time>, mut query: Query<(&mut PowerUp, &Position, &mut Path)>) {
    let dt = time.delta_seconds();
    for (mut power_up, pos, mut path) in query.iter_mut() {
        power_up.time_since_created += dt;
        *path = circle!(power_up_radius(power_up.time_since_created), pos).path;
    }
}

fn collect_power_ups(
    mut commands: Commands,
    mut player: Query<(&Position, &mut PlayerState, &mut Health), With<Player>>,
    power_ups: Query<(Entity, &Position, &PowerUp, &PowerUpType), With<PowerUp>>
) {
    let (player_pos, mut player_state, mut player_health) = player.single_mut();
    let player_pos = to_vec2!(player_pos);
    for (id, pos, power_up, power_up_type) in power_ups.iter() {
        let power_up_radius  = power_up_radius(power_up.time_since_created);
        if to_vec2!(pos).distance(player_pos) < PLAYER_RADIUS + power_up_radius {
            match power_up_type {
                PowerUpType::Heal => {
                    let heal_amount = player_health.max_hp / 5;
                    player_health.heal(heal_amount);
                },
                PowerUpType::Boost => player_state.boost_time_left += 3.0,
            };
            commands.entity(id).despawn();
        }
    }
}

fn player_ranged_attack(
    mut commands: Commands,
    window: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    query: Query<(Entity, &PlayerStats, &PlayerState, &Position), With<Player>>
) {
    let (player_id, stats, state, pos) = query.single();

    let (camera, transform) = camera.single();
    let Some(relative_mouse_coords): Option<Vec2> = window.single()
        .cursor_position()
        .and_then(|mouse| camera.viewport_to_world_2d(transform, mouse))
        .map(|mouse| mouse - Vec2::new(pos.x, pos.y))
    else { return };

    if !state.ranged_attack_timer.just_finished() {
        return;
    }

    let damage = state.current_weapon.adjusted_damage(stats.ranged_attack_damage);
    let velocity = {
        let length = relative_mouse_coords.length();
        let x = relative_mouse_coords.x / length;
        let y = relative_mouse_coords.y / length;
        let scale = stats.ranged_attack_speed;

        Vec2::new(x * scale, y * scale)
            * state.current_weapon.speed_multiplier()
    };
    let location = pos.into();

    match state.current_weapon {
        PlayerWeapon::Normal => {
            let pierce_left = stats.ranged_attack_pierce;
            let radius = BASE_PROJECTILE_RADIUS;
            commands.spawn((
                RunningObject,
                PlayerProjectile,
                Projectile { damage, velocity, location, radius, pierce_left, last_entity_hit: player_id },
                circle!(radius, pos),
                Fill::color(YELLOW_GREEN),
                Stroke::new(BLACK, 5f32),
            ));
        },
        PlayerWeapon::Burst => {
            for burst_info in distribute_burst(stats.ranged_attack_damage, stats.ranged_attack_pierce) {
                let location = velocity * burst_info.offset_dist + location;
                let radius = BASE_PROJECTILE_RADIUS * 0.5;
                commands.spawn((
                    RunningObject,
                    PlayerProjectile,
                    Projectile {
                        damage: burst_info.damage,
                        velocity: velocity.rotate(Vec2::from_angle(burst_info.offset_rad)),
                        location,
                        radius,
                        pierce_left: burst_info.pierce,
                        last_entity_hit: player_id,
                    },
                    circle!(radius, location),
                    Fill::color(GREEN_YELLOW),
                    Stroke::new(BLACK, 3f32),
                ));
            }
        },
        PlayerWeapon::Splash => {
            let radius = BASE_PROJECTILE_RADIUS * 1.7;
            commands.spawn((
                RunningObject,
                PlayerProjectile,
                Projectile {
                    damage, velocity, location, radius,
                    pierce_left: 0,
                    last_entity_hit: player_id,
                },
                SplashProjectile {
                    damage: state.current_weapon.adjusted_splash_damage(stats.ranged_attack_damage),
                    range: 100.0,
                },
                circle!(radius, location),
                Fill::color(PURPLE_800),
                Stroke::new(BLACK, 7f32),
            ));
        },
    }
}

fn check_and_resolve_player_death(mut commands: Commands, query: Query<&Health, With<Player>>) {
    let player_hp = query.single().current_hp;
    if player_hp == 0 {
        commands.remove_resource::<Running>();
        commands.insert_resource(Transition::new(Running, LoseScreen));
    }
}

fn move_projectiles(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Projectile, &mut Path)>,
    player_loc: Query<&Position, With<Player>>,
) {
    let player_loc = player_loc.single();

    for (id, mut proj, mut path) in query.iter_mut() {
        let approx_x = proj.location.x as isize;
        let approx_y = proj.location.y as isize;

        if (approx_x - player_loc.x as isize).abs() > 2000
            || (approx_y - player_loc.y as isize).abs() > 1500 {
            commands.entity(id).despawn();
            continue;
        }

        let vel = proj.velocity;
        proj.location += vel * time.delta_seconds();
        *path = circle!(proj.radius, proj.location).path;
    }
}

fn update_player_color(mut query: Query<(&mut Fill, &Health), With<Player>>) {

    let (mut fill, health) = query.single_mut();

    let health_percent = (health.current_health() as f32) / (health.max_health() as f32);

    fn mix(min: f32, max: f32, percent: f32) -> f32 {
        (min * (1.0 - percent)) + (max * percent)
    }

    fill.color = Srgba::rgb(
        mix(PLAYER_COLOR_NO_HP.red, PLAYER_COLOR_MAX_HP.red, health_percent),
        mix(PLAYER_COLOR_NO_HP.green, PLAYER_COLOR_MAX_HP.green, health_percent),
        mix(PLAYER_COLOR_NO_HP.blue, PLAYER_COLOR_MAX_HP.blue, health_percent),
    ).into();
}

fn spawn_wave_if_no_enemies(
    mut commands: Commands,
    mut player_upgrade_counter: ResMut<PlayerUpgradeCounter>,
    mut player_upgrade_counter_text: Query<&mut Text, (With<PlayerUpgradeCounterText>, Without<WaveCounterText>)>,
    mut wave_counter: ResMut<WaveCounter>,
    mut wave_counter_text: Query<&mut Text, (With<WaveCounterText>, Without<PlayerUpgradeCounterText>)>,
    mut player: Query<(&Position, &mut Health, &PlayerStats), With<Player>>,
    query: Query<&Enemy>
) {
    use num_traits::float::FloatConst;

    // If the query returns Some(_), then there are still enemies left.
    // Therefore, a new wave should not be spawned.
    if query.iter().len() != 0 {
        return;
    }

    // Since we're spawning a new wave, increment the wave counter.
    // Note: WaveCounter::default() is 0 so the first wave will be 1.
    wave_counter.0 += 1;
    let mut wave_counter_text = wave_counter_text.single_mut();
    *wave_counter_text = Text::from_section(format!("Wave {}", wave_counter.0), TextStyle::default());

    player_upgrade_counter.add_unused();
    *player_upgrade_counter_text.single_mut() =
        Text::from_section(player_upgrade_counter.display_text(), TextStyle::default());

    let (player_pos, mut player_health, player_stats) = player.single_mut();
    player_health.heal(player_stats.end_of_round_heal);

    let enemies_to_spawn = 8 + (2 * wave_counter.0);
    const ENEMY_DISTANCE_MIN: f32 = 400f32;
    let enemy_distance_max: f32 = 800f32 + (20f32 * wave_counter.0 as f32);
    let enemy_distance_diff = enemy_distance_max - ENEMY_DISTANCE_MIN;
    for _ in 0..enemies_to_spawn {
        let angle_radians: f32 = random::<f32>() * f32::PI() * 2.0;
        let distance: f32 = ENEMY_DISTANCE_MIN + (random::<f32>() * enemy_distance_diff);
        let pos = Position::new(
            player_pos.x + (angle_radians.cos() * distance),
            player_pos.y + (angle_radians.sin() * distance),
        );
        let stats = EnemyStats::new(
            9 + wave_counter.0 as usize,
            1.0 * f32::powi(0.95, (wave_counter.0 - 1) as i32),
            if wave_counter.0 < 5
                || (wave_counter.0 < 10 && random::<f32>() < 0.9)
                || (wave_counter.0 < 25 && random::<f32>() < 0.8)
                || (wave_counter.0 < 20 && random::<f32>() < 0.6)
                || wave_counter.0 > 20
            { 0 }
            else {
                (wave_counter.0 - 5) as usize
                * f32::powi(1.05, (wave_counter.0 - 10) as i32) as usize
            },
            1.5,
            200.0 * f32::powi(1.05, wave_counter.0 as i32),
            75.0 * f32::powi(1.05, wave_counter.0 as i32),
        );
        commands.spawn((
            RunningObject,
            NoFrustumCulling, // prevent weird invisibility
            Enemy,
            pos,
            Health::new(24 + (2*wave_counter.0 as usize)),
            stats,
            EnemyState::from_enemy_stats(stats).with_random_ranged_timer(),
            circle!(ENEMY_RADIUS, pos),
            Fill::color(ENEMY_COLOR),
            Stroke::new(BLACK, 3.0),
        ));
    }
}

fn apply_player_upgrade(stats: &mut PlayerStats, health: &mut Health, upgrade: PlayerUpgrade) {
    match upgrade {
        PlayerUpgrade::AttackUpgrade => {
            stats.ranged_attack_damage += 2;
            stats.ranged_attack_cooldown *= 0.95;
            stats.ranged_attack_pierce += 1;
            stats.close_attack_damage += 10;
        },
        PlayerUpgrade::HealthUpgrade => {
            health.add_max_hp(20);
            health.heal(health.max_health() / 5);
            stats.end_of_round_heal += 4;
        },
        PlayerUpgrade::SpeedUpgrade => {
            stats.movement_speed *= 1.15;
            stats.ranged_attack_speed *= 1.2;
        },
    }
}

fn distribute_burst(damage: usize, pierce: usize) -> Vec<BurstInfo> {
    let total = damage * pierce;

    if total <= 200 {
        let mut bursts = Vec::with_capacity(5);
        for i in -2i8..=2 {
            bursts.push(BurstInfo {
                damage: (total + 3) / 6,
                pierce: 2,
                offset_rad: 0.15 * f32::from(i),
                offset_dist: 0.0,
            })
        }
        return bursts;
    }

    if total <= 600 {
        let mut bursts = Vec::with_capacity(7);
        for i in -3i8..=3 {
            bursts.push(BurstInfo {
                damage: damage * 2 / 3,
                pierce: pierce / 4,
                offset_rad: 0.1 * f32::from(i),
                offset_dist: 0.0,
            })
        }
        return bursts;
    }

    let mut bursts = Vec::with_capacity(9);
    for i in -4i8..=4 {
        bursts.push(BurstInfo {
            damage: damage / 5,
            pierce: pierce - 4,
            offset_rad: 0.12 * f32::from(i),
            offset_dist: 0.0,
        })
    }

    bursts
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
struct Guide;
#[derive(Resource)]
#[derive(Debug, Copy, Clone)]
struct LoseScreen;

#[derive(Component)]
#[derive(Debug, Copy, Clone)]
struct MainMenuItem;
#[derive(Component)]
#[derive(Debug, Copy, Clone)]
struct LoseScreenItem;
#[derive(Component)]
#[derive(Debug, Copy, Clone)]
struct RunningObject;
#[derive(Component)]
#[derive(Debug, Copy, Clone)]
struct GuideItem;

#[derive(Resource)]
#[derive(Debug, Copy, Clone, Default)]
struct Transition<From: Resource, To: Resource> {
    _make_the_compiler_happy: std::marker::PhantomData<(From, To)>
}

#[derive(Resource)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Default)]
struct WaveCounter(pub isize);
#[derive(Component)]
#[derive(Debug, Copy, Clone)]
struct WaveCounterText;

#[derive(Resource)]
#[derive(Debug, Copy, Clone, Default)]
struct HighScore(pub isize);

#[derive(Component)]
#[derive(Debug, Copy, Clone)]
struct PlayerHealthText;

#[derive(Resource)]
#[derive(Debug, Copy, Clone, Default)]
struct PlayerUpgradeCounter {
    pub attack_upgrades: usize,
    pub health_upgrades: usize,
    pub speed_upgrades: usize,
    pub unused_upgrades: usize,
}
#[derive(Component)]
#[derive(Debug, Copy, Clone)]
struct PlayerUpgradeCounterText;

#[derive(Component)]
#[derive(Debug, Copy, Clone, Default)]
struct Player;

#[derive(Component)]
#[derive(Debug, Copy, Clone, Default)]
struct Enemy;

#[derive(Component)]
#[derive(Debug, Copy, Clone)]
struct PlayerProjectile;
#[derive(Component)]
#[derive(Debug, Copy, Clone)]
struct EnemyProjectile;

#[derive(Component)]
#[derive(Debug, Clone, PartialEq)]
struct Projectile {
    pub damage: usize,
    pub velocity: Vec2,
    pub location: Vec2,
    pub pierce_left: usize,
    pub last_entity_hit: Entity,
    pub radius: f32,
}

#[derive(Component)]
#[derive(Debug, Copy, Clone, PartialEq, Default)]
struct SplashProjectile {
    pub damage: usize,
    pub range: f32,
}

#[derive(Component)]
#[derive(Debug, Copy, Clone, PartialEq, Default)]
struct SplashProjectileResidual {
    pub damage: usize,
    pub radius: f32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct BurstInfo {
    pub damage: usize,
    pub pierce: usize,
    pub offset_rad: f32,
    pub offset_dist: f32,
}

#[derive(Component)]
#[derive(Debug, Clone, PartialEq)]
struct SplashResidue(pub Timer);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum PlayerUpgrade {
    HealthUpgrade,
    AttackUpgrade,
    SpeedUpgrade
}

#[derive(Resource)]
#[derive(Debug, Clone)]
struct PowerUpSpawnTimer(pub Timer);

#[derive(Component)]
#[derive(Debug, Copy, Clone, PartialEq)]
struct PowerUp { pub time_since_created: f32 }

#[derive(Component)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum PowerUpType {
    Heal,
    Boost,
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
    end_of_round_heal: usize,
}

#[derive(Debug, Copy, Clone, Default)]
enum PlayerWeapon {
    #[default] Normal,
    Burst,
    Splash,
}

#[derive(Component)]
#[derive(Debug, Clone)]
struct PlayerState {
    close_attack_timer: Timer,
    ranged_attack_timer: Timer,
    current_weapon: PlayerWeapon,
    boost_time_left: f32,
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

impl<From: Resource, To: Resource> Transition<From, To> {
    fn new(_from: From, _to: To) -> Self {
        Self { _make_the_compiler_happy: std::marker::PhantomData }
    }
}

impl Default for PowerUpSpawnTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(4f32, TimerMode::Repeating))
    }
}

impl PowerUpType {
    fn color(&self) -> Srgba {
        match self {
            PowerUpType::Heal => PINK_600,
            PowerUpType::Boost => YELLOW_400,
        }
    }

    fn random() -> Self {
        match random::<u8>() % 2 {
            0 => PowerUpType::Heal,
            1 => PowerUpType::Boost,
            _ => unreachable!("any number mod 2 is either 0 or 1"),
        }
    }
}

impl PlayerUpgradeCounter {
    fn add_unused(&mut self) {
        self.unused_upgrades += 1;
    }

    fn add_upgrade(&mut self, player_upgrade: PlayerUpgrade) {
        match player_upgrade {
            PlayerUpgrade::HealthUpgrade => self.health_upgrades += 1,
            PlayerUpgrade::SpeedUpgrade => self.speed_upgrades += 1,
            PlayerUpgrade::AttackUpgrade => self.attack_upgrades += 1,
        }
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn display_text(&self) -> String {
        format!(
            "Unused upgrades: {}\n\
            Health upgrades [1]: {}\n\
            Attack upgrades [2]: {}\n\
            Speed upgrades [3]: {}\n",
            self.unused_upgrades,
            self.health_upgrades,
            self.attack_upgrades,
            self.speed_upgrades,
        )
    }
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    fn distance(&self, other: &Self) -> f32 {
        let this: Vec2 = self.into();
        let other: Vec2 = other.into();
        this.distance(other)
    }
}

impl PlayerWeapon {
    fn cycle_left(&mut self) {
        *self = match self {
            Self::Normal => Self::Splash,
            Self::Splash => Self::Burst,
            Self::Burst => Self::Normal,
        }
    }
    pub fn cycle_right(&mut self) {
        *self = match self {
            Self::Normal => Self::Burst,
            Self::Burst => Self::Splash,
            Self::Splash => Self::Normal,
        }
    }

    pub fn cooldown_multiplier(&self) -> f32 {
        match self {
            Self::Normal => 1.2,
            Self::Burst => 1.8,
            Self::Splash => 2.5,
        }
    }
    pub fn adjusted_damage(&self, base_damage: usize) -> usize {
        match self {
            Self::Normal => base_damage - 3,
            Self::Burst => base_damage,
            Self::Splash => base_damage + 2,
        }
    }
    pub fn speed_multiplier(&self) -> f32 {
        match self {
            Self::Normal => 0.85,
            Self::Burst => 1.2,
            Self::Splash => 2.0,
        }
    }
    pub fn adjusted_splash_damage(&self, base_damage: usize) -> usize {
        match self {
            Self::Splash => base_damage * 22 / 10,
            _ => 0
        }
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
            current_weapon: PlayerWeapon::default(),
            boost_time_left: 0.0,
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

    fn randomize_ranged_timer(&mut self) {
        use std::time::Duration;

        self.close_attack_timer.set_elapsed(Duration::from_secs_f32(
            random::<f32>() * self.close_attack_timer.duration().as_secs_f32()
        ));
        self.ranged_attack_timer.set_elapsed(Duration::from_secs_f32(
            random::<f32>() * self.ranged_attack_timer.duration().as_secs_f32()
        ));
    }

    fn with_random_ranged_timer(mut self) -> Self {
        self.randomize_ranged_timer();
        self
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
        self.current_hp = self.max_hp.min(self.current_hp + heal_amount)
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

            ranged_attack_damage: 6usize,
            ranged_attack_cooldown: 0.15f32,
            ranged_attack_pierce: 4usize,
            ranged_attack_speed: 450f32,

            movement_speed: 150f32,
            end_of_round_heal: 5usize,
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
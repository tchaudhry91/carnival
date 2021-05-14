use bevy::core::FixedTimestep;
use bevy::prelude::*;
use bevy::render::pass::ClearColor;
use rand::prelude::random;

const ARENA_WIDTH: u32 = 20;
const ARENA_HEIGHT: u32 = 20;

#[derive(PartialEq, Copy, Clone, Debug)]
struct Position {
    x: i32,
    y: i32,
}

struct Size {
    width: f32,
    height: f32,
}

impl Size {
    pub fn square(x: f32) -> Self {
        Self {
            width: x,
            height: x,
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Action {
    Idle,
    Move,
    Dig,
    Build,
}

#[derive(PartialEq, Copy, Clone)]
pub enum Direction {
    Up,
    Down,
    Right,
    Left,
}

struct Player {
    face_direction: Direction,
    action: Action,
    has_rock: bool,
}
struct Wall;

struct Materials {
    player_material: Handle<ColorMaterial>,
    wall_material: Handle<ColorMaterial>,
}

fn setup(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.insert_resource(Materials {
        player_material: materials.add(Color::rgb(0.7, 0.7, 0.7).into()),
        wall_material: materials.add(Color::rgb(1., 0., 0.).into()),
    });
}

fn spawn_player(mut commands: Commands, materials: Res<Materials>) {
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.player_material.clone(),
            sprite: Sprite::new(Vec2::new(20.0, 20.0)),
            ..Default::default()
        })
        .insert(Player {
            action: Action::Idle,
            face_direction: Direction::Up,
            has_rock: false,
        })
        .insert(Position { x: 1, y: 1 })
        .insert(Size::square(0.5));
}

fn spawn_boundaries(mut commands: Commands, materials: Res<Materials>) {
    let mut boundary_positions: Vec<Position> = Vec::new();
    for x in 0..ARENA_WIDTH {
        boundary_positions.push(Position { x: x as i32, y: 0 });
        boundary_positions.push(Position {
            x: x as i32,
            y: ARENA_HEIGHT as i32 - 1,
        })
    }
    for y in 1..ARENA_HEIGHT - 1 {
        boundary_positions.push(Position { x: 0, y: y as i32 });
        boundary_positions.push(Position {
            x: ARENA_WIDTH as i32 - 1,
            y: y as i32,
        });
    }
    while let Some(p) = boundary_positions.pop() {
        commands
            .spawn_bundle(SpriteBundle {
                material: materials.wall_material.clone(),
                sprite: Sprite::new(Vec2::new(20.0, 20.0)),
                ..Default::default()
            })
            .insert(Wall)
            .insert(p)
            .insert(Size::square(0.8));
    }
}

fn spawn_walls(
    mut commands: Commands,
    materials: Res<Materials>,
    walls: Query<&Position, With<Wall>>,
    players: Query<&Position, With<Player>>,
) {
    let mut target_position = Position { x: 0, y: 0 };
    // Do not spawn on top of an existing wall or player
    'outer: loop {
        target_position.x = (random::<f32>() * ARENA_WIDTH as f32) as i32;
        target_position.y = (random::<f32>() * ARENA_HEIGHT as f32) as i32;
        for p in players.iter() {
            if p == &target_position {
                continue 'outer;
            }
        }
        for p in walls.iter() {
            if p == &target_position {
                continue 'outer;
            }
        }
        break;
    }
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.wall_material.clone(),
            sprite: Sprite::new(Vec2::new(20.0, 20.0)),
            ..Default::default()
        })
        .insert(Wall)
        .insert(target_position)
        .insert(Size::square(0.8));
}

fn size_scaling(windows: Res<Windows>, mut q: Query<(&Size, &mut Sprite)>) {
    let window = windows.get_primary().unwrap();
    for (sprite_size, mut sprite) in q.iter_mut() {
        sprite.size = Vec2::new(
            sprite_size.width / ARENA_WIDTH as f32 * window.width() as f32,
            sprite_size.height / ARENA_WIDTH as f32 * window.height() as f32,
        );
    }
}

fn position_translation(windows: Res<Windows>, mut q: Query<(&Position, &mut Transform)>) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }
    let window = windows.get_primary().unwrap();
    for (pos, mut transform) in q.iter_mut() {
        transform.translation = Vec3::new(
            convert(pos.x as f32, window.width() as f32, ARENA_WIDTH as f32),
            convert(pos.y as f32, window.height() as f32, ARENA_HEIGHT as f32),
            0.0,
        )
    }
}

pub struct PlayerActionPlugin;

#[derive(SystemLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub enum PlayerActions {
    Input,
    InputValidation,
    MoveAction,
    DigAction,
    BuildAction,
}

impl Plugin for PlayerActionPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(
            player_input
                .system()
                .label(PlayerActions::Input)
                .before(PlayerActions::InputValidation),
        )
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(0.05))
                .with_system(
                    validate_player_action
                        .system()
                        .label(PlayerActions::InputValidation)
                        .before(PlayerActions::MoveAction),
                )
                .with_system(
                    player_move_action
                        .system()
                        .label(PlayerActions::MoveAction)
                        .before(PlayerActions::DigAction),
                )
                .with_system(
                    player_dig_action
                        .system()
                        .label(PlayerActions::DigAction)
                        .before(PlayerActions::BuildAction),
                )
                .with_system(
                    player_build_action
                        .system()
                        .label(PlayerActions::BuildAction),
                ),
        );
    }
}

#[derive(SystemLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub enum PlayerMovement {
    Input,
    Movement,
}

fn player_input(keyboard_input: Res<Input<KeyCode>>, mut player_positions: Query<&mut Player>) {
    for mut p in player_positions.iter_mut() {
        if keyboard_input.just_pressed(KeyCode::J) || keyboard_input.just_pressed(KeyCode::Down) {
            p.face_direction = Direction::Down;
            p.action = Action::Move;
        }
        if keyboard_input.just_pressed(KeyCode::K) || keyboard_input.just_pressed(KeyCode::Up) {
            p.face_direction = Direction::Up;
            p.action = Action::Move;
        }
        if keyboard_input.just_pressed(KeyCode::L) || keyboard_input.just_pressed(KeyCode::Right) {
            p.face_direction = Direction::Right;
            p.action = Action::Move;
        }
        if keyboard_input.just_pressed(KeyCode::H) || keyboard_input.just_pressed(KeyCode::Left) {
            p.face_direction = Direction::Left;
            p.action = Action::Move;
        }
        if keyboard_input.just_pressed(KeyCode::Space) {
            if !p.has_rock {
                p.action = Action::Dig;
            }
            if p.has_rock {
                p.action = Action::Build;
            }
        }
    }
}

fn validate_player_action(
    mut players: Query<(&Position, &mut Player)>,
    walls: Query<&Position, With<Wall>>,
) {
    for (pos, mut player) in players.iter_mut() {
        match player.action {
            Action::Move => match player.face_direction {
                Direction::Down => {
                    let target_position = Position {
                        x: pos.x,
                        y: pos.y - 1,
                    };
                    for w in walls.iter() {
                        if &target_position == w {
                            player.action = Action::Idle;
                        }
                    }
                }
                Direction::Up => {
                    let target_position = Position {
                        x: pos.x,
                        y: pos.y + 1,
                    };
                    for w in walls.iter() {
                        if &target_position == w {
                            player.action = Action::Idle;
                        }
                    }
                }
                Direction::Left => {
                    let target_position = Position {
                        x: pos.x - 1,
                        y: pos.y,
                    };
                    for w in walls.iter() {
                        if &target_position == w {
                            player.action = Action::Idle;
                        }
                    }
                }
                Direction::Right => {
                    let target_position = Position {
                        x: pos.x + 1,
                        y: pos.y,
                    };
                    for w in walls.iter() {
                        if &target_position == w {
                            player.action = Action::Idle;
                        }
                    }
                }
            },
            Action::Dig => match player.face_direction {
                Direction::Down => {
                    let target_position = Position {
                        x: pos.x,
                        y: pos.y - 1,
                    };
                    player.action = Action::Idle;
                    for w in walls.iter() {
                        if &target_position == w {
                            player.action = Action::Dig;
                            break;
                        }
                    }
                }
                Direction::Up => {
                    let target_position = Position {
                        x: pos.x,
                        y: pos.y + 1,
                    };
                    player.action = Action::Idle;
                    for w in walls.iter() {
                        if &target_position == w {
                            player.action = Action::Dig;
                            break;
                        }
                    }
                }
                Direction::Left => {
                    let target_position = Position {
                        x: pos.x - 1,
                        y: pos.y,
                    };
                    player.action = Action::Idle;
                    for w in walls.iter() {
                        if &target_position == w {
                            player.action = Action::Dig;
                            break;
                        }
                    }
                }
                Direction::Right => {
                    let target_position = Position {
                        x: pos.x + 1,
                        y: pos.y,
                    };
                    player.action = Action::Idle;
                    for w in walls.iter() {
                        if &target_position == w {
                            player.action = Action::Dig;
                            break;
                        }
                    }
                }
            },
            Action::Build => match player.face_direction {
                Direction::Down => {
                    let target_position = Position {
                        x: pos.x,
                        y: pos.y - 1,
                    };
                    for w in walls.iter() {
                        if &target_position == w {
                            player.action = Action::Idle;
                        }
                    }
                }
                Direction::Up => {
                    let target_position = Position {
                        x: pos.x,
                        y: pos.y + 1,
                    };
                    for w in walls.iter() {
                        if &target_position == w {
                            player.action = Action::Idle;
                        }
                    }
                }
                Direction::Left => {
                    let target_position = Position {
                        x: pos.x - 1,
                        y: pos.y,
                    };
                    for w in walls.iter() {
                        if &target_position == w {
                            player.action = Action::Idle;
                        }
                    }
                }
                Direction::Right => {
                    let target_position = Position {
                        x: pos.x + 1,
                        y: pos.y,
                    };
                    for w in walls.iter() {
                        if &target_position == w {
                            player.action = Action::Idle;
                        }
                    }
                }
            },
            Action::Idle => {}
        }
    }
}

fn player_move_action(mut player_positions: Query<(&mut Position, &mut Player)>) {
    for (mut pos, mut player) in player_positions.iter_mut() {
        if player.action == Action::Move {
            match player.face_direction {
                Direction::Down => {
                    pos.y -= 1;
                }
                Direction::Up => {
                    pos.y += 1;
                }
                Direction::Left => {
                    pos.x -= 1;
                }
                Direction::Right => {
                    pos.x += 1;
                }
            }
            player.action = Action::Idle;
        }
    }
}

fn player_dig_action(
    mut commands: Commands,
    mut players: Query<(&Position, &mut Player)>,
    walls: Query<(Entity, &Position, &Wall)>,
) {
    for (position, mut player) in players.iter_mut() {
        let mut pos = *position;
        if player.action == Action::Dig {
            match player.face_direction {
                Direction::Down => {
                    pos.y -= 1;
                }
                Direction::Up => {
                    pos.y += 1;
                }
                Direction::Left => {
                    pos.x -= 1;
                }
                Direction::Right => {
                    pos.x += 1;
                }
            }
            for (e, wpos, _w) in walls.iter() {
                if wpos == &pos {
                    commands.entity(e).despawn();
                    player.has_rock = true;
                }
            }
            player.action = Action::Idle;
        }
    }
}

fn player_build_action(
    mut commands: Commands,
    materials: Res<Materials>,
    mut players: Query<(&Position, &mut Player)>,
) {
    for (position, mut player) in players.iter_mut() {
        let mut pos = *position;
        if player.action == Action::Build {
            match player.face_direction {
                Direction::Down => {
                    pos.y -= 1;
                }
                Direction::Up => {
                    pos.y += 1;
                }
                Direction::Left => {
                    pos.x -= 1;
                }
                Direction::Right => {
                    pos.x += 1;
                }
            }
            commands
                .spawn_bundle(SpriteBundle {
                    material: materials.wall_material.clone(),
                    sprite: Sprite::new(Vec2::new(20.0, 20.0)),
                    ..Default::default()
                })
                .insert(Wall)
                .insert(pos)
                .insert(Size::square(0.8));
            player.has_rock = false;
            player.action = Action::Idle;
        }
    }
}

fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            title: "Carnival".to_string(),
            width: 400.0,
            height: 400.0,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
        .add_startup_system(setup.system())
        .add_startup_stage("player_loader", SystemStage::single(spawn_player.system()))
        .add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new()
                .with_system(position_translation.system())
                .with_system(size_scaling.system()),
        )
        .add_startup_stage(
            "boundary_loader",
            SystemStage::single(spawn_boundaries.system()),
        )
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(1.0))
                .with_system(spawn_walls.system()),
        )
        .add_plugin(PlayerActionPlugin)
        .add_plugins(DefaultPlugins)
        .run();
}

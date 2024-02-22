use bevy::ecs::component::Component;

#[derive(Component)]
pub struct Dead;

#[derive(Component)]
pub struct Velocity {
    pub dx: f32,
    pub dy: f32,
}

#[derive(Component)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Component)]
pub struct Health {
    pub current: u8,
    pub max: u8,
}

#[derive(Component)]
pub struct Dirty;

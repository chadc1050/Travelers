use bevy::prelude::*;

#[derive(Clone, Component)]
pub struct Inventory {
    pub items: Vec<Item>,
}

#[derive(Clone)]
pub struct Item {
    name: String,
}

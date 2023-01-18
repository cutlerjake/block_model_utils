use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct BlockCoordinates {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(
    Debug, PartialEq, Copy, Clone, Hash, Eq, Default, Serialize, Deserialize, PartialOrd, Ord,
)]
pub struct BlockIndex {
    pub i: usize,
    pub j: usize,
    pub k: usize,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct BlockSize {
    pub x_size: f32,
    pub y_size: f32,
    pub z_size: f32,
}
pub trait BlockInterface: Clone + PartialEq + for<'a> Deserialize<'a> {
    //coordinates of block in space
    fn coordinates(&self) -> BlockCoordinates;

    //dimensions of block
    fn size(&self) -> BlockSize;

    //index
    fn index(&self) -> BlockIndex;
    fn set_index(&mut self, ind: BlockIndex);
}

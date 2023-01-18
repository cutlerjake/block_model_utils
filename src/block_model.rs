use ndarray::Array3;
use num;

use crate::block::{BlockCoordinates, BlockIndex, BlockInterface, BlockSize};

use std::error::Error;

pub trait BlockDependenceInterface {
    fn inds<B: BlockInterface>(&self, mdl: &BlockModel<B>, ind: BlockIndex) -> Vec<BlockIndex>;
}

#[derive(Copy, Clone, Debug, Default, Hash)]
pub struct SquarePreds;

impl SquarePreds {
    const PREDS: isize = -1;
    const SUCCS: isize = 1;
}

impl BlockDependenceInterface for SquarePreds {
    fn inds<B: BlockInterface>(&self, mdl: &BlockModel<B>, ind: BlockIndex) -> Vec<BlockIndex> {
        let k = ind.k + 1;
        if k >= mdl.blocks.raw_dim()[2] {
            return vec![];
        };

        let i_low = num::clamp(ind.i as i64 - 1, 0, mdl.blocks.shape()[0] as i64) as usize;
        let i_high = num::clamp(ind.i + 2, 0, mdl.blocks.shape()[0]);

        let j_low = num::clamp(ind.j as i64 - 1, 0, mdl.blocks.shape()[1] as i64) as usize;
        let j_high = num::clamp(ind.j + 2, 0, mdl.blocks.shape()[1]);

        let mut inds = Vec::with_capacity(9);
        for i in i_low..i_high {
            for j in j_low..j_high {
                inds.push(BlockIndex { i, j, k });
            }
        }
        inds
    }
}

pub struct SquareSuccs;

impl BlockDependenceInterface for SquareSuccs {
    fn inds<B: BlockInterface>(&self, mdl: &BlockModel<B>, ind: BlockIndex) -> Vec<BlockIndex> {
        if ind.k == 0 {
            return vec![];
        }
        let k = ind.k - 1;

        let i_low = num::clamp(ind.i as i64 - 1, 0, mdl.blocks.shape()[0] as i64) as usize;
        let i_high = num::clamp(ind.i + 2, 0, mdl.blocks.shape()[0]);

        let j_low = num::clamp(ind.j as i64 - 1, 0, mdl.blocks.shape()[1] as i64) as usize;
        let j_high = num::clamp(ind.j + 2, 0, mdl.blocks.shape()[1]);

        let mut inds = Vec::with_capacity(9);
        for i in i_low..i_high {
            for j in j_low..j_high {
                inds.push(BlockIndex { i, j, k });
            }
        }
        inds
    }
}

pub struct SquareAdj;
impl BlockDependenceInterface for SquareAdj {
    fn inds<B: BlockInterface>(&self, mdl: &BlockModel<B>, ind: BlockIndex) -> Vec<BlockIndex> {
        let k = ind.k;

        let i_low = num::clamp(ind.i as i64 - 1, 0, mdl.blocks.shape()[0] as i64) as usize;
        let i_high = num::clamp(ind.i + 2, 0, mdl.blocks.shape()[0]);

        let j_low = num::clamp(ind.j as i64 - 1, 0, mdl.blocks.shape()[1] as i64) as usize;
        let j_high = num::clamp(ind.j + 2, 0, mdl.blocks.shape()[1]);

        let mut inds = Vec::with_capacity(9);
        for i in i_low..i_high {
            for j in j_low..j_high {
                if mdl.block(BlockIndex { i, j, k }).is_some() {
                    inds.push(BlockIndex { i, j, k });
                }
            }
        }
        inds
    }
}

#[derive(Debug)]
pub struct BlockModel<B>
where
    B: BlockInterface,
{
    pub blocks: Array3<Option<B>>,
}

impl<B> BlockModel<B>
where
    B: BlockInterface,
{
    fn gen_inds(
        blocks: &Vec<B>,
        origin: BlockCoordinates,
        block_size: BlockSize,
    ) -> Vec<BlockIndex> {
        blocks
            .iter()
            .map(|ub| {
                let coords = ub.coordinates();
                let i = (coords.x - origin.x) / block_size.x_size;
                let j = (coords.y - origin.y) / block_size.y_size;
                let k = (coords.z - origin.z) / block_size.z_size;

                //incorrect block size
                assert!(i.fract() == 0.0 && j.fract() == 0.0 && k.fract() == 0.0);

                BlockIndex {
                    i: i as usize,
                    j: j as usize,
                    k: k as usize,
                }
            })
            .collect::<Vec<_>>()
    }

    pub fn from_unindexed(blocks: Vec<B>) -> Self {
        let mut blocks = blocks;

        //get origin of model
        let (min_x, min_y, min_z) = blocks.iter().fold(
            (f32::MAX, f32::MAX, f32::MAX),
            |(mut x, mut y, mut z), b| {
                x = x.min(b.coordinates().x);
                y = y.min(b.coordinates().y);
                z = z.min(b.coordinates().z);

                (x, y, z)
            },
        );

        let origin = BlockCoordinates {
            x: min_x,
            y: min_y,
            z: min_z,
        };

        //get block dims and ensure all same size
        let dims = match blocks.as_slice() {
            [head, tail @ ..] => tail
                .iter()
                .all(|b| head.size() == b.size())
                .then(|| head.size()),
            _ => None,
        };

        if dims == None {
            panic!()
        };

        //Generate indexed block set
        let inds = Self::gen_inds(&blocks, origin, dims.unwrap());

        blocks
            .iter_mut()
            .zip(inds.iter())
            .for_each(|(b, ind)| b.set_index(*ind));

        Self::from_indexed(blocks, inds)
    }

    pub fn from_indexed(blocks: Vec<B>, inds: Vec<BlockIndex>) -> Self {
        //Find model dimensions
        let (max_i, max_j, max_k) = inds.iter().fold((0, 0, 0), |(mut i, mut j, mut k), ib| {
            i = i.max(ib.i);
            j = j.max(ib.j);
            k = k.max(ib.k);
            (i, j, k)
        });

        //create array to store blocks
        let mut block_arr = Array3::from_elem((max_i + 1, max_j + 1, max_k + 1), None);

        //populate bm
        blocks.into_iter().zip(inds).for_each(|(b, ind)| {
            let BlockIndex { i, j, k } = ind;
            block_arr[[i, j, k]] = Some(b);
        });

        Self { blocks: block_arr }
    }

    pub fn from_unindexed_csv(file: String) -> Result<Self, Box<dyn Error>> {
        //create reader and storage for blocks
        let mut rdr = csv::Reader::from_path(file)?;

        let mut blocks = Vec::new();

        //create blocks
        for result in rdr.deserialize() {
            let block: B = result?;
            blocks.push(block);
        }

        Ok(Self::from_unindexed(blocks))
    }

    pub fn block(&self, ind: BlockIndex) -> &Option<B> {
        &self.blocks[[ind.i, ind.j, ind.k]]
    }

    pub fn block_mut(&mut self, ind: BlockIndex) -> &mut Option<B> {
        &mut self.blocks[[ind.i, ind.j, ind.k]]
    }

    pub fn dependent_block_inds<BDI: BlockDependenceInterface>(
        &self,
        ind: BlockIndex,
        bdi: BDI,
    ) -> Vec<BlockIndex> {
        bdi.inds(&self, ind)
    }
    pub fn from_indexed_csv(file: String) -> Result<Self, Box<dyn Error>> {
        //create reader and storage for blocks
        let mut rdr = csv::Reader::from_path(file)?;
        let mut blocks = Vec::new();
        let mut inds = Vec::new();

        //create blocks
        for result in rdr.deserialize() {
            let block: B = result?;
            inds.push(block.index());
            blocks.push(block);
        }

        Ok(Self::from_indexed(blocks, inds))
    }
}

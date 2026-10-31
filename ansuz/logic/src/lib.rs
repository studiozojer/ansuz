//! The entry experience is native Contract UI. Keep record identity independent
//! of its projection; no game canvas or voxel assets are mounted by the playground.
use exact_game::*;
pub mod records;

#[derive(Default, Args)]
pub struct Options {}

pub struct Ansuz;
impl Game for Ansuz {
    const ID: &'static str = "ansuz";
    type Args = Options;
    fn setup(_world: &mut World, _args: &Options) {}
    fn tick(_world: &mut World, _input: &Input, _args: &Options) {}
}

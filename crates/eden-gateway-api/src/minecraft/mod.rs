pub mod block_pos;
pub mod dimension;

// It is named that way because in player.data's NBT structure has
// `playerGameType` inside where it determines the player's game mode
// (display name for game type).
//
// See at: https://minecraft.wiki/w/Player.dat_format
pub mod game_type;

pub use self::block_pos::BlockPos;
pub use self::dimension::{Dimension, McDimension};
pub use self::game_type::GameType;

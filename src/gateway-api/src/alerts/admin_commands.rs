use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::minecraft::{BlockPos, Dimension, GameType};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "server", derive(validator::Validate))]
pub struct AdminCommandAlert {
    #[cfg_attr(feature = "server", validate(length(min = 2)))]
    pub command: String,
    pub executor: Executor,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Executor {
    Console,
    Player(ExecutorPlayerInfo),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExecutorPlayerInfo {
    pub dimension: Dimension,
    pub gamemode: GameType,
    pub position: BlockPos,
    pub username: String,
    pub uuid: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization_with_console_executor() {
        let alert = AdminCommandAlert {
            command: "/tell Notch I have secrets".into(),
            executor: Executor::Console,
        };
        insta::assert_json_snapshot!(alert);
    }

    #[test]
    fn test_serialization_with_player_executor() {
        let alert = AdminCommandAlert {
            command: "/tell Notch I have secrets".into(),
            executor: Executor::Player(ExecutorPlayerInfo {
                dimension: Dimension::OVERWORLD,
                gamemode: GameType::Survival,
                position: BlockPos::ZERO,
                username: "jebs".into(),
                uuid: Uuid::nil(),
            }),
        };
        insta::assert_json_snapshot!(alert);
    }
}

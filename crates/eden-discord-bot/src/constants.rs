use splinter::ShardingRange;
use std::time::Duration;
use twilight_cache_inmemory::ResourceType;
use twilight_gateway::{EventTypeFlags, Intents};

pub const EVENT_TYPE_FLAGS: EventTypeFlags = EventTypeFlags::READY
    .union(EventTypeFlags::GUILD_CREATE)
    .union(EventTypeFlags::MESSAGE_CREATE);

pub const INTENTS: Intents = Intents::DIRECT_MESSAGES
    .union(Intents::GUILDS)
    .union(Intents::GUILD_MESSAGES)
    .union(Intents::MESSAGE_CONTENT);

pub const SHARDING_RANGE: ShardingRange = ShardingRange::ONE;
pub const SUPERVISOR_CHECK_INTERVAL: Duration = Duration::from_mins(1);

/// Welcome message sent to the primary guild when Eden first joins it.
pub const PRIMARY_GUILD_WELCOME_MESSAGE: &str = concat!(
    "
    **Thank you for choosing Eden as your primary Discord bot for your Minecraft server needs!** :laughing:

    **Please bare mind that this bot is in development phase. Bugs are expected to occur at anytime**. If you encountered bugs/issues with this bot, don't hesitate to report us at: ",
    env!("CARGO_PKG_REPOSITORY"),
    "/issues",
);

pub mod swearing_police {
    use eden_text_handling::swearing::RustrictType;
    use std::sync::LazyLock;

    // rustrict's Type does not support any constant functions that allow for bitshift unions.
    pub const SWEARING_POLICE_THRESHOLD: LazyLock<RustrictType> =
        LazyLock::new(|| RustrictType::OFFENSIVE | RustrictType::PROFANE | RustrictType::SEVERE);

    // rustrict's Type does not support any constant functions that allow for bitshift unions.
    pub const CENSORED_THRESHOLD: LazyLock<RustrictType> = LazyLock::new(|| {
        RustrictType::INAPPROPRIATE
            | RustrictType::EVASIVE
            | RustrictType::OFFENSIVE
            | RustrictType::SEVERE
    });
}

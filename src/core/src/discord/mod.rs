use eden_twilight::{PERMISSIONS_TO_SEND, is_everyone_role};
use twilight_model::{
    guild::Permissions,
    id::Id,
    id::marker::{ChannelMarker, GuildMarker, RoleMarker, UserMarker},
};
use twilight_util::permission_calculator::PermissionCalculator;

use crate::Kernel;

impl Kernel {
    #[must_use]
    pub fn can_send_alerts_to_discord(
        &self,
        alert_channel_id: Id<ChannelMarker>,
        additional_perms: Option<Permissions>,
    ) -> bool {
        let primary_guild_id = self.config.bot.primary_guild.id;
        let Some(current_user) = self.discord_cache.current_user() else {
            return false;
        };

        self.calculate_channel_permissions(primary_guild_id, current_user.id, alert_channel_id)
            .contains(PERMISSIONS_TO_SEND | additional_perms.unwrap_or_else(Permissions::empty))
    }

    pub fn calculate_channel_permissions(
        &self,
        guild_id: Id<GuildMarker>,
        bot_user_id: Id<UserMarker>,
        channel_id: Id<ChannelMarker>,
    ) -> Permissions {
        let mut dump = Vec::new();

        let result = self.get_permission_calculator(guild_id, bot_user_id, &mut dump);
        let Some(calculator) = result else {
            return Permissions::all();
        };

        let Some(channel) = self.discord_cache.channel(channel_id) else {
            return Permissions::all();
        };

        if let Some(overwrites) = channel.permission_overwrites.as_ref() {
            calculator.in_channel(channel.kind, overwrites)
        } else {
            calculator.in_channel(channel.kind, &[])
        }
    }

    /// Builds a [`PermissionCalculator`] from cache for a guild member, incorporating
    /// the `@everyone` role permissions and the member's assigned role permissions.
    ///
    /// If no `@everyone` role is found in `guild_roles`, its permissions default
    /// to [`Permissions::empty`].
    ///
    /// The `bot_member_roles` parameter is required due to the nature of [`PermissionCalculator`],
    /// holding a reference to a slice containing the member’s roles and their permissions.
    ///
    /// You must pass an empty mutable [`Vec`] that has no capacity pre-allocated.
    pub fn get_permission_calculator<'r>(
        &self,
        guild_id: Id<GuildMarker>,
        bot_user_id: Id<UserMarker>,
        bot_member_roles: &'r mut Vec<(Id<RoleMarker>, Permissions)>,
    ) -> Option<PermissionCalculator<'r>> {
        let member = self.discord_cache.member(guild_id, bot_user_id)?;
        let everyone_perms = self
            .discord_cache
            .guild_roles(guild_id)?
            .iter()
            .filter_map(|&id| self.discord_cache.role(id))
            .find(|v| is_everyone_role(v))
            .map(|v| v.permissions)
            .unwrap_or_else(Permissions::empty);

        *bot_member_roles = Vec::with_capacity(member.roles().len());

        for &id in member.roles() {
            if let Some(role) = self.discord_cache.role(id) {
                bot_member_roles.push((role.id, role.permissions));
            }
        }

        Some(PermissionCalculator::new(
            guild_id,
            bot_user_id,
            everyone_perms,
            bot_member_roles,
        ))
    }
}

use twilight_model::{
    guild::{Permissions, Role},
    id::Id,
    id::marker::{GuildMarker, RoleMarker, UserMarker},
};
use twilight_util::permission_calculator::PermissionCalculator;

pub mod http;

/// Required channel permissions to send a message on Discord.
///
/// Take note, that this value contains the minimum requirements to
/// send a plain text message on Discord. If you want to send attachments
/// or any message that requires additional permissions (e.g. sending attachments),
/// please include it.
pub const PERMISSIONS_TO_SEND: Permissions =
    Permissions::VIEW_CHANNEL.union(Permissions::SEND_MESSAGES);

/// Builds a [`PermissionCalculator`] for a guild member, incorporating the
/// `@everyone` role permissions and the member's assigned role permissions.
///
/// If no `@everyone` role is found in `guild_roles`, its permissions default
/// to [`Permissions::empty`].
///
/// The `member_roles` slice should contain only the roles assigned to the
/// member. Use [`resolve_member_roles`] to resolve a member's role IDs into
/// `(Id<RoleMarker>, Permissions)` pairs suitable for passing here.
#[must_use = "calculating permissions is only useful if they're used"]
pub fn build_calculator<'a>(
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    guild_roles: &'a [Role],
    member_roles: &'a [(Id<RoleMarker>, Permissions)],
) -> PermissionCalculator<'a> {
    let everyone_perms = find_everyone_role(guild_roles)
        .map(|v| v.permissions)
        .unwrap_or_else(Permissions::empty);

    PermissionCalculator::new(guild_id, user_id, everyone_perms, member_roles)
}

/// Returns true of whether a specific role is `@everyone` role.
#[must_use]
pub fn is_everyone_role(role: &Role) -> bool {
    const EVERYONE_ROLE_NAME: &str = "@everyone";
    role.name == EVERYONE_ROLE_NAME
}

/// Returns the `@everyone` role from a guild's role list if it exists.
#[must_use]
pub fn find_everyone_role(roles: &[Role]) -> Option<&Role> {
    roles.iter().find(|r| is_everyone_role(r))
}

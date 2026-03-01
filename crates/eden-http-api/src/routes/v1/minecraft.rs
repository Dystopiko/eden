use eden_database::primary_guild::minecraft::DbMinecraftAccount;
use eden_kernel::Kernel;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug)]
pub struct GetAccountInfo {
    pub id: Uuid,
}

pub async fn join(kernel: Arc<Kernel>, info: GetAccountInfo) {
    let mut conn = kernel.primary_db.acquire().await.unwrap();
    let Some(account) = DbMinecraftAccount::find_by_uuid(&mut conn, info.id)
        .await
        .unwrap()
    else {
        panic!("cannot find mc account by path");
    };
}

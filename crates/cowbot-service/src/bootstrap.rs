use cowbot_core::error::CowResult;
use cowbot_core::identity::{Membership, Role, User, Workspace};
use cowbot_store::Store;

/// The local user and workspace created or reused on startup.
#[derive(Debug, Clone)]
pub struct LocalContext {
    pub user: User,
    pub workspace: Workspace,
}

/// Bootstrap a local single-user workspace.
///
/// The operation is idempotent: existing users and workspaces are reused, and
/// the first user is always ensured to be an owner of the first workspace.
pub async fn bootstrap_local(store: &Store) -> CowResult<LocalContext> {
    bootstrap_local_named(store, "Local Owner", "Default Workspace").await
}

/// Bootstrap with custom owner and workspace display names.
pub async fn bootstrap_local_named(
    store: &Store,
    owner_name: &str,
    workspace_name: &str,
) -> CowResult<LocalContext> {
    let user = match store.list_users().await?.into_iter().next() {
        Some(existing) => existing,
        None => {
            let created = User::new(owner_name, "local@cowbot.local", "");
            store.create_user(&created).await?;
            created
        }
    };

    let workspace = match store.list_workspaces().await?.into_iter().next() {
        Some(existing) => existing,
        None => {
            let created = Workspace::new(workspace_name);
            store.create_workspace(&created).await?;
            created
        }
    };

    store
        .add_member(&Membership::new(
            user.id.clone(),
            workspace.id.clone(),
            Role::Owner,
        ))
        .await?;

    Ok(LocalContext { user, workspace })
}

use std::path::PathBuf;

use codex_protocol::ThreadId;
use codex_state::SqliteConfig;
use codex_thread_store::LocalThreadStore;
use codex_thread_store::LocalThreadStoreConfig;
use codex_thread_store::RolloutMigrationMode;
use codex_thread_store::RolloutMigrationOptions;
use codex_thread_store::RolloutMigrationStatus;
use codex_utils_absolute_path::AbsolutePathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let codex_home = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: codex-history-migrate CODEX_HOME THREAD_ID [THREAD_ID ...]")?;
    let thread_ids = args
        .map(|value| ThreadId::from_string(&value))
        .collect::<Result<Vec<_>, _>>()?;
    if thread_ids.is_empty() {
        return Err("at least one thread id is required".into());
    }

    let sqlite_home = AbsolutePathBuf::from_absolute_path(&codex_home)?;
    let sqlite = SqliteConfig::from_sqlite_home(sqlite_home);
    let state_db = codex_state::StateRuntime::init(sqlite.clone(), "openai".to_string()).await?;
    let store = LocalThreadStore::new(
        LocalThreadStoreConfig {
            codex_home: codex_home.clone(),
            sqlite,
            default_model_provider_id: "openai".to_string(),
        },
        Some(state_db),
    );
    let report = store
        .migrate_rollouts(RolloutMigrationOptions {
            mode: RolloutMigrationMode::Apply,
            thread_ids,
            max_mib_per_second: 1024,
        })
        .await?;
    println!("{}", serde_json::to_string_pretty(&report)?);

    let failed = report.outcomes.iter().any(|outcome| {
        !matches!(
            outcome.status,
            RolloutMigrationStatus::Migrated | RolloutMigrationStatus::AlreadyPaginated
        )
    });
    if failed {
        std::process::exit(2);
    }
    Ok(())
}

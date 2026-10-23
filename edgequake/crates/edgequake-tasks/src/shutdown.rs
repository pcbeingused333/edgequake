//! SPEC-083 X-31: cooperative shutdown drain budget.
//!
//! Long PDF / LLM work must not block process exit forever. Operators set
//! `EDGEQUAKE_SHUTDOWN_DRAIN_SECS` (default 30). After the budget, workers are
//! aborted and the HTTP serve future is dropped.

use std::time::Duration;

/// Env var for shutdown drain budget (seconds).
pub const SHUTDOWN_DRAIN_SECS_ENV: &str = "EDGEQUAKE_SHUTDOWN_DRAIN_SECS";

/// Default drain budget when the env var is unset.
pub const DEFAULT_SHUTDOWN_DRAIN_SECS: u64 = 30;

/// Resolve the shutdown drain budget from the environment.
///
/// Clamped to `[1, 3600]` so a typo cannot disable drain (0) or hang deploys
/// for hours.
pub fn shutdown_drain_budget() -> Duration {
    let secs = std::env::var(SHUTDOWN_DRAIN_SECS_ENV)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_SHUTDOWN_DRAIN_SECS)
        .clamp(1, 3600);
    Duration::from_secs(secs)
}

/// SPEC-171: sqlx `Pool::close` rejects new acquires with this text.
pub fn is_closed_pool_message(message: &str) -> bool {
    message.contains("closed pool")
}

/// Workers finish, then pools close. Callers pass the production futures.
pub async fn drain_workers_then_close_pools<W, C>(workers: W, close_pools: C)
where
    W: std::future::Future<Output = ()>,
    C: std::future::Future<Output = ()>,
{
    workers.await;
    close_pools.await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_drain_default_is_30s() {
        // Avoid poisoning parallel tests: only assert clamp math on a local parse path.
        let secs = "not-a-number"
            .parse::<u64>()
            .ok()
            .unwrap_or(DEFAULT_SHUTDOWN_DRAIN_SECS)
            .clamp(1, 3600);
        assert_eq!(secs, 30);
        assert_eq!(DEFAULT_SHUTDOWN_DRAIN_SECS, 30);
    }

    #[test]
    fn shutdown_drain_clamp_rejects_zero() {
        assert_eq!(0u64.clamp(1, 3600), 1);
    }

    #[tokio::test]
    async fn t171_38_workers_stop_before_pools_close() {
        let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let workers = {
            let order = order.clone();
            async move {
                order.lock().expect("order").push("worker_shutdown");
            }
        };
        let pools = {
            let order = order.clone();
            async move {
                order.lock().expect("order").push("pool_close");
            }
        };
        drain_workers_then_close_pools(workers, pools).await;
        assert_eq!(
            order.lock().expect("order").as_slice(),
            ["worker_shutdown", "pool_close"]
        );
    }

    #[test]
    fn t171_12_closed_pool_is_shutdown() {
        assert!(is_closed_pool_message(
            "attempted to acquire a connection on a closed pool"
        ));
        assert!(!is_closed_pool_message("connection refused"));
    }
}

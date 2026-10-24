//! `edgequake originals` — move BYTEA originals into the object store.
//!
//! The default verb is `plan`. Clear is a separate step and needs `--yes`.

use std::env;
use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use edgequake_storage::originals::{
    assert_maint_role, claim_statement_timeout, clear_owners, connect_maint_pool, copy_owners_with,
    open_from_env, pending_copy_count, plan_text, redact_database_url, restore_owners, status_text,
    verify_owners, KindFilter, OriginalsError, OwnerKind, PgLedger, RunOptions, Scope, StoreMode,
};
use uuid::Uuid;

const USAGE: &str = "\
usage:
  edgequake originals [plan]                 count rows and the space VACUUM FULL needs
  edgequake originals copy                   PUT each BYTEA that has no ref
  edgequake originals verify                 hash every ref against the object
  edgequake originals migrate [--clear --yes] copy then verify; clear only with both flags
  edgequake originals clear --yes            null verified BYTEA; mode must be object
  edgequake originals restore                write verified objects back into BYTEA
  edgequake originals status                 counts by state

flags:
  --concurrency N    parallel owners (default 4)
  --limit N          stop after N owners
  --kind pdf|original|asset|markdown|both|all
  --workspace UUID
  --yes              required for clear and migrate --clear
  --clear            only on migrate, after a clean verify
  --help             print this text
";

#[derive(Debug)]
struct Flags {
    verb: String,
    limit: Option<u64>,
    kind: KindFilter,
    workspace: Option<Uuid>,
    yes: bool,
    clear: bool,
    concurrency: usize,
}

pub async fn run(args: &[String]) -> anyhow::Result<()> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let flags = match parse(args) {
        Ok(flags) => flags,
        Err(message) => {
            eprintln!("{USAGE}");
            eprintln!("{message}");
            eprintln!("originals_abort=usage");
            std::process::exit(2);
        }
    };
    if let Err(err) = execute(flags).await {
        let code = err.exit_code();
        eprintln!("{err}");
        eprintln!("originals_abort={}", err.class_name());
        std::process::exit(code);
    }
    Ok(())
}

fn parse(args: &[String]) -> Result<Flags, String> {
    let mut verb = "plan".to_string();
    let mut limit = None;
    let mut kind = KindFilter::All;
    let mut workspace = None;
    let mut yes = false;
    let mut clear = false;
    let mut concurrency: usize = 4;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if !arg.starts_with("--") && verb == "plan" && i == 0 {
            verb = arg.clone();
            i += 1;
            continue;
        }
        match arg.as_str() {
            "--yes" => yes = true,
            "--clear" => clear = true,
            "--concurrency" | "--limit" | "--kind" | "--workspace" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| format!("{arg} needs a value"))?;
                match arg.as_str() {
                    "--concurrency" => {
                        concurrency = value
                            .parse::<usize>()
                            .map_err(|_| "--concurrency is not a number".to_string())?;
                        if concurrency == 0 {
                            return Err("--concurrency must be at least 1".into());
                        }
                    }
                    "--limit" => {
                        limit = Some(
                            value
                                .parse::<u64>()
                                .map_err(|_| "--limit is not a number".to_string())?,
                        );
                    }
                    "--kind" => {
                        kind = match value.as_str() {
                            "pdf" => KindFilter::One(OwnerKind::Pdf),
                            "original" => KindFilter::One(OwnerKind::Original),
                            "asset" => KindFilter::One(OwnerKind::Asset),
                            "markdown" => KindFilter::One(OwnerKind::Markdown),
                            "both" => KindFilter::Both,
                            "all" => KindFilter::All,
                            other => {
                                return Err(format!(
                                    "--kind {other} is not pdf, original, asset, markdown, both, or all"
                                ))
                            }
                        };
                    }
                    "--workspace" => {
                        workspace = Some(
                            value
                                .parse()
                                .map_err(|_| "--workspace is not a UUID".to_string())?,
                        );
                    }
                    _ => {}
                }
                i += 1;
            }
            other if other.starts_with("--") => return Err(format!("unknown flag {other}")),
            other if i == 0 => verb = other.to_string(),
            other => return Err(format!("unexpected argument {other}")),
        }
        i += 1;
    }
    finish(verb, limit, kind, workspace, yes, clear, concurrency)
}

fn finish(
    verb: String,
    limit: Option<u64>,
    kind: KindFilter,
    workspace: Option<Uuid>,
    yes: bool,
    clear: bool,
    concurrency: usize,
) -> Result<Flags, String> {
    match verb.as_str() {
        "plan" | "copy" | "verify" | "migrate" | "clear" | "restore" | "status" => {}
        other => return Err(format!("unknown originals subcommand '{other}'")),
    }
    if clear && verb != "migrate" {
        return Err("--clear is only valid on migrate".into());
    }
    if verb == "clear" && !yes {
        return Err("clear drops the database copy; pass --yes".into());
    }
    if verb == "migrate" && clear && !yes {
        return Err("migrate --clear drops the database copy; pass --yes".into());
    }
    Ok(Flags {
        verb,
        limit,
        kind,
        workspace,
        yes,
        clear,
        concurrency,
    })
}

async fn execute(flags: Flags) -> Result<(), OriginalsError> {
    let database_url = env::var("EDGEQUAKE_ORIGINALS_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .map_err(|_| {
            OriginalsError::Config("DATABASE_URL is required for edgequake originals".into())
        })?;
    let opened = open_from_env().await?;
    let scope = Scope {
        kind: flags.kind,
        workspace: flags.workspace,
        limit: flags.limit,
    };
    let mode_name = match opened.mode {
        StoreMode::Database => "database",
        StoreMode::Object => "object",
    };
    let store_url = env::var("EDGEQUAKE_ORIGINAL_STORE_URL").unwrap_or_else(|_| "unset".into());
    match flags.verb.as_str() {
        "plan" | "status" => {
            let pool = connect_maint_pool(&database_url).await?;
            assert_maint_role(&pool).await?;
            let ledger = PgLedger::new(pool);
            let text = if flags.verb == "status" {
                status_text(
                    &ledger,
                    &scope,
                    &redact_database_url(&database_url),
                    &store_url,
                    mode_name,
                )
                .await?
            } else {
                plan_text(
                    &ledger,
                    &scope,
                    &redact_database_url(&database_url),
                    &store_url,
                    mode_name,
                )
                .await?
            };
            print!("{text}");
            if opened.mode == StoreMode::Database {
                println!("note       server mode is database; clear would be refused");
            }
            Ok(())
        }
        "copy" | "verify" | "migrate" | "clear" | "restore" => {
            let client = opened.client.ok_or_else(|| {
                OriginalsError::Config(
                    "this verb writes to the store; set EDGEQUAKE_ORIGINAL_STORE=object and EDGEQUAKE_ORIGINAL_STORE_URL"
                        .into(),
                )
            })?;
            let pool = connect_maint_pool(&database_url).await?;
            assert_maint_role(&pool).await?;
            let _timeout = claim_statement_timeout(&pool).await?;
            let count_pool = pool.clone();
            let ledger = PgLedger::new(pool);
            let opts = RunOptions {
                prefix: client.prefix.clone(),
                sse: client.sse,
                mode: opened.mode,
                scope: scope.clone(),
                yes: flags.yes,
                concurrency: flags.concurrency,
            };
            match flags.verb.as_str() {
                "copy" | "migrate" => {
                    let pending = pending_copy_count(&count_pool, &scope).await?;
                    let stderr_tty = std::io::stderr().is_terminal();
                    if pending == 0 {
                        eprintln!(
                            "migrate    store={store}  concurrency={concurrency}  pending=0",
                            store = store_url,
                            concurrency = flags.concurrency,
                        );
                        eprintln!("verify    nothing new to copy; checking objects already stored");
                    } else {
                        eprintln!(
                            "migrate    store={store}  concurrency={concurrency}  pending={pending}",
                            store = store_url,
                            concurrency = flags.concurrency,
                        );
                    }
                    let on_copied = move |n: u64| {
                        if stderr_tty {
                            eprint!("\rcopy    {n}/{pending}   ");
                            let _ = std::io::stderr().flush();
                        } else if n.is_multiple_of(100) || n == pending {
                            eprintln!("copy    {n}/{pending}");
                        }
                    };
                    let report =
                        copy_owners_with(&ledger, client.blobs.as_ref(), &opts, &on_copied).await?;
                    if stderr_tty {
                        eprintln!();
                    }
                    print!("{}", report.text);
                    if flags.verb == "migrate" {
                        if report.copied > 0 {
                            eprintln!("verify    hashing {copied} objects", copied = report.copied);
                        }
                        verify_owners(&ledger, client.blobs.as_ref(), &scope).await?;
                        if flags.clear {
                            let text = clear_owners(&ledger, client.blobs.as_ref(), &opts).await?;
                            print!("{text}");
                        }
                    }
                    Ok(())
                }
                "verify" => {
                    let n = verify_owners(&ledger, client.blobs.as_ref(), &scope).await?;
                    println!("verified {n}");
                    Ok(())
                }
                "clear" => {
                    let text = clear_owners(&ledger, client.blobs.as_ref(), &opts).await?;
                    print!("{text}");
                    Ok(())
                }
                "restore" => {
                    let n = restore_owners(&ledger, client.blobs.as_ref(), &scope).await?;
                    println!("restored {n}");
                    println!("next    switch EDGEQUAKE_ORIGINAL_STORE=database after you confirm the bytes");
                    Ok(())
                }
                _ => unreachable!(),
            }
        }
        _ => Err(OriginalsError::Usage("unknown verb".into())),
    }
}

#[allow(dead_code)]
pub fn usage() -> &'static str {
    USAGE
}

#[allow(dead_code)]
pub fn exit_ok() -> ExitCode {
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn clear_without_yes_is_usage_before_any_write() {
        let err = parse(&args(&["clear"])).unwrap_err();
        assert!(err.contains("--yes"));
    }

    #[test]
    fn migrate_clear_without_yes_is_usage() {
        let err = parse(&args(&["migrate", "--clear"])).unwrap_err();
        assert!(err.contains("--yes"));
    }

    #[test]
    fn clear_flag_is_only_valid_on_migrate() {
        let err = parse(&args(&["copy", "--clear", "--yes"])).unwrap_err();
        assert!(err.contains("only valid on migrate"));
    }

    #[test]
    fn plan_is_the_default_verb() {
        let flags = parse(&args(&[])).unwrap();
        assert_eq!(flags.verb, "plan");
        assert_eq!(flags.concurrency, 4);
        assert_eq!(flags.kind, KindFilter::All);
    }

    #[test]
    fn both_stays_pdf_and_original_and_all_includes_images_and_markdown() {
        let both = parse(&args(&["plan", "--kind", "both"])).unwrap();
        assert_eq!(both.kind, KindFilter::Both);
        assert!(both.kind.includes(OwnerKind::Pdf));
        assert!(!both.kind.includes(OwnerKind::Asset));
        assert!(!both.kind.includes(OwnerKind::Markdown));
        let all = parse(&args(&["plan", "--kind", "all"])).unwrap();
        assert!(all.kind.includes(OwnerKind::Asset));
        assert!(all.kind.includes(OwnerKind::Markdown));
        let asset = parse(&args(&["copy", "--kind", "asset"])).unwrap();
        assert_eq!(asset.kind, KindFilter::One(OwnerKind::Asset));
        let markdown = parse(&args(&["copy", "--kind", "markdown"])).unwrap();
        assert_eq!(markdown.kind, KindFilter::One(OwnerKind::Markdown));
    }
}

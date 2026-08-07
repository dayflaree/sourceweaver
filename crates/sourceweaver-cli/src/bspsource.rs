use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
struct BspSourceManagedAsset {
    id: &'static str,
    name: &'static str,
    platform: &'static str,
    version: &'static str,
    url: &'static str,
    sha256: &'static str,
    size: u64,
    release_url: &'static str,
    source_tag: &'static str,
    java_note: &'static str,
}

const BSPSOURCE_MANAGED_VERSION: &str = "v1.4.8";
const BSPSOURCE_RELEASE_URL: &str = "https://github.com/ata4/bspsrc/releases/tag/v1.4.8";
const BSPSOURCE_LICENSE_URL: &str = "https://github.com/ata4/bspsrc/blob/master/LICENSE.md";
const BSPSOURCE_SOURCE_TAG: &str = "https://github.com/ata4/bspsrc/tree/v1.4.8";
const BSPSOURCE_ASSETS: &[BspSourceManagedAsset] = &[
    BspSourceManagedAsset {
        id: "jar-only",
        name: "bspsrc-jar-only.zip",
        platform: "portable-java",
        version: BSPSOURCE_MANAGED_VERSION,
        url: "https://github.com/ata4/bspsrc/releases/download/v1.4.8/bspsrc-jar-only.zip",
        sha256: "d5effc38b78c4f60f8eb4f9be1db717bb808227a9013f82d20f34860a128b0e7",
        size: 7_414_395,
        release_url: BSPSOURCE_RELEASE_URL,
        source_tag: BSPSOURCE_SOURCE_TAG,
        java_note: "Requires Java 24+ according to upstream v1.4.8 release notes.",
    },
    BspSourceManagedAsset {
        id: "linux",
        name: "bspsrc-linux.zip",
        platform: "linux",
        version: BSPSOURCE_MANAGED_VERSION,
        url: "https://github.com/ata4/bspsrc/releases/download/v1.4.8/bspsrc-linux.zip",
        sha256: "646c3dcc7cdc58650a96ad985a0e093bf3ef1e53b43e01aae01168910d14a32d",
        size: 49_422_392,
        release_url: BSPSOURCE_RELEASE_URL,
        source_tag: BSPSOURCE_SOURCE_TAG,
        java_note: "No system Java required according to upstream v1.4.8 release notes.",
    },
    BspSourceManagedAsset {
        id: "windows",
        name: "bspsrc-windows.zip",
        platform: "windows",
        version: BSPSOURCE_MANAGED_VERSION,
        url: "https://github.com/ata4/bspsrc/releases/download/v1.4.8/bspsrc-windows.zip",
        sha256: "6297f7fa567adbaf72738b0a707ff45916edc920c66543098408e4b9d41ec4a9",
        size: 45_781_672,
        release_url: BSPSOURCE_RELEASE_URL,
        source_tag: BSPSOURCE_SOURCE_TAG,
        java_note: "No system Java required according to upstream v1.4.8 release notes.",
    },
];

pub fn command(args: &[String]) -> Result<(), String> {
    let Some(subcommand) = args.first().map(String::as_str) else {
        print_help();
        return Ok(());
    };
    match subcommand {
        "manifest" => manifest_command(&args[1..]),
        "policy" => policy_command(&args[1..]),
        "cache-path" => cache_path_command(&args[1..]),
        "verify" => verify_command(&args[1..]),
        "download" => download_command(&args[1..]),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => Err(format!(
            "unknown bspsource subcommand `{other}`. Run `sourceweaver bspsource help`."
        )),
    }
}

fn manifest_command(args: &[String]) -> Result<(), String> {
    let json = args.iter().any(|arg| arg == "--json");
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&manifest_json())
                .map_err(|error| format!("failed to encode BSPSource manifest JSON: {error}"))?
        );
    } else {
        println!("BSPSource managed manifest: {BSPSOURCE_MANAGED_VERSION}");
        println!("release: {BSPSOURCE_RELEASE_URL}");
        println!("license: {BSPSOURCE_LICENSE_URL}");
        for asset in BSPSOURCE_ASSETS {
            println!(
                "{}\t{}\t{} bytes\tsha256:{}\t{}",
                asset.id, asset.name, asset.size, asset.sha256, asset.url
            );
        }
    }
    Ok(())
}

fn policy_command(args: &[String]) -> Result<(), String> {
    let json = args.iter().any(|arg| arg == "--json");
    let policy = policy_json();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&policy)
                .map_err(|error| format!("failed to encode BSPSource policy JSON: {error}"))?
        );
    } else {
        println!("BSPSource managed download policy");
        println!("- BSPSource itself is listed upstream under the Unlicense.");
        println!("- Upstream lists Apache-2.0 and BSD-3-Clause dependencies in LICENSE.md.");
        println!("- Source Weaver does not bundle or redistribute BSPSource assets.");
        println!(
            "- Downloads are user-initiated, version-pinned, checksum-verified, and cache-only."
        );
        println!("- User-selected local BSPSource launcher/jar paths remain supported.");
        println!(
            "- Real BSPSource validation is only claimed when the real tool is actually run and evidence is recorded."
        );
    }
    Ok(())
}

fn cache_path_command(args: &[String]) -> Result<(), String> {
    let options = parse_options(args)?;
    let asset = selected_asset(options.asset.as_deref())?;
    let cache_dir = options.cache_dir.unwrap_or_else(default_cache_dir);
    let path = cache_file_path(&cache_dir, asset);
    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "asset": asset_json(asset),
                "cache_dir": cache_dir.display().to_string(),
                "path": path.display().to_string(),
            }))
            .map_err(|error| format!("failed to encode BSPSource cache JSON: {error}"))?
        );
    } else {
        println!("{}", path.display());
    }
    Ok(())
}

fn verify_command(args: &[String]) -> Result<(), String> {
    let options = parse_options(args)?;
    let file = options.file.ok_or("bspsource verify needs --file <zip>")?;
    let asset = if options.expected_sha256.is_none() {
        Some(selected_asset(options.asset.as_deref())?)
    } else {
        options
            .asset
            .as_deref()
            .map(|asset| selected_asset(Some(asset)))
            .transpose()?
    };
    let expected_sha256 = options
        .expected_sha256
        .or_else(|| asset.map(|asset| asset.sha256.to_string()))
        .ok_or("bspsource verify needs --asset <id> or --sha256 <hex>")?;
    let (size, actual_sha256) = sha256_file(&file)?;
    let ok = actual_sha256.eq_ignore_ascii_case(&expected_sha256)
        && asset.map(|asset| asset.size == size).unwrap_or(true);
    let report = serde_json::json!({
        "ok": ok,
        "file": file.display().to_string(),
        "asset": asset.map(asset_json),
        "expected_sha256": expected_sha256,
        "actual_sha256": actual_sha256,
        "size": size,
        "expected_size": asset.map(|asset| asset.size),
    });
    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .map_err(|error| format!("failed to encode BSPSource verify JSON: {error}"))?
        );
    } else if ok {
        println!("BSPSource checksum verified: {}", file.display());
    } else {
        println!("BSPSource checksum verification failed: {}", file.display());
    }
    if ok {
        Ok(())
    } else {
        Err("BSPSource checksum/size verification failed".to_string())
    }
}

fn download_command(args: &[String]) -> Result<(), String> {
    let options = parse_options(args)?;
    let asset = selected_asset(options.asset.as_deref())?;
    let cache_dir = options.cache_dir.unwrap_or_else(default_cache_dir);
    let path = cache_file_path(&cache_dir, asset);
    if path.exists() {
        let (size, sha256) = sha256_file(&path)?;
        if size == asset.size && sha256 == asset.sha256 {
            return print_download_result(
                true,
                "cached",
                asset,
                &cache_dir,
                &path,
                size,
                &sha256,
                options.json,
            );
        }
    }
    if !options.accept_download_policy {
        return Err(
            "managed BSPSource download requires --accept-download-policy after reviewing `sourceweaver bspsource policy`; local tool paths remain supported".to_string(),
        );
    }
    fs::create_dir_all(path.parent().unwrap_or(&cache_dir)).map_err(|error| {
        format!(
            "failed to create BSPSource cache directory {}: {error}",
            path.parent().unwrap_or(&cache_dir).display()
        )
    })?;
    let partial_path = path.with_extension("zip.partial");
    let response = ureq::get(asset.url)
        .call()
        .map_err(|error| format!("failed to download {}: {error}", asset.url))?;
    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to read BSPSource download stream: {error}"))?;
    fs::write(&partial_path, &bytes).map_err(|error| {
        format!(
            "failed to write BSPSource partial download {}: {error}",
            partial_path.display()
        )
    })?;
    let (size, sha256) = sha256_file(&partial_path)?;
    if size != asset.size || sha256 != asset.sha256 {
        let _ = fs::remove_file(&partial_path);
        return Err(format!(
            "downloaded BSPSource asset failed checksum/size verification: expected {} bytes sha256:{}, got {} bytes sha256:{}",
            asset.size, asset.sha256, size, sha256
        ));
    }
    fs::rename(&partial_path, &path).map_err(|error| {
        format!(
            "failed to move BSPSource download into cache {}: {error}",
            path.display()
        )
    })?;
    print_download_result(
        true,
        "downloaded",
        asset,
        &cache_dir,
        &path,
        size,
        &sha256,
        options.json,
    )
}

#[allow(clippy::too_many_arguments)]
fn print_download_result(
    ok: bool,
    status: &str,
    asset: &BspSourceManagedAsset,
    cache_dir: &Path,
    path: &Path,
    size: u64,
    sha256: &str,
    json: bool,
) -> Result<(), String> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": ok,
                "status": status,
                "asset": asset_json(asset),
                "cache_dir": cache_dir.display().to_string(),
                "path": path.display().to_string(),
                "size": size,
                "sha256": sha256,
            }))
            .map_err(|error| format!("failed to encode BSPSource download JSON: {error}"))?
        );
    } else {
        println!(
            "BSPSource {status}: {} ({} bytes sha256:{sha256})",
            path.display(),
            size
        );
    }
    Ok(())
}

#[derive(Debug, Default)]
struct Options {
    asset: Option<String>,
    cache_dir: Option<PathBuf>,
    file: Option<PathBuf>,
    expected_sha256: Option<String>,
    accept_download_policy: bool,
    json: bool,
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--asset" => {
                cursor += 1;
                options.asset = Some(args.get(cursor).ok_or("--asset needs a value")?.clone());
            }
            "--cache-dir" => {
                cursor += 1;
                options.cache_dir = Some(PathBuf::from(
                    args.get(cursor).ok_or("--cache-dir needs a path")?,
                ));
            }
            "--file" => {
                cursor += 1;
                options.file = Some(PathBuf::from(
                    args.get(cursor).ok_or("--file needs a path")?,
                ));
            }
            "--sha256" => {
                cursor += 1;
                options.expected_sha256 = Some(normalize_sha256(
                    args.get(cursor).ok_or("--sha256 needs a value")?,
                )?);
            }
            "--accept-download-policy" => options.accept_download_policy = true,
            "--json" => options.json = true,
            value if value.starts_with('-') => {
                return Err(format!("unknown bspsource option `{value}`"));
            }
            value => {
                if options.asset.is_none() {
                    options.asset = Some(value.to_string());
                } else if options.file.is_none() {
                    options.file = Some(PathBuf::from(value));
                } else {
                    return Err(format!("unexpected bspsource argument `{value}`"));
                }
            }
        }
        cursor += 1;
    }
    Ok(options)
}

fn selected_asset(asset: Option<&str>) -> Result<&'static BspSourceManagedAsset, String> {
    let requested = asset.unwrap_or(default_asset_id());
    let normalized = requested
        .trim()
        .to_ascii_lowercase()
        .replace('_', "-")
        .replace(".zip", "");
    let id = match normalized.as_str() {
        "jar" | "jar-only" | "portable" | "bspsrc-jar-only" => "jar-only",
        "linux" | "bspsrc-linux" => "linux",
        "win" | "windows" | "bspsrc-windows" => "windows",
        other => other,
    };
    BSPSOURCE_ASSETS
        .iter()
        .find(|asset| asset.id == id)
        .ok_or_else(|| {
            format!("unknown BSPSource asset `{requested}`. choices: jar-only, linux, windows")
        })
}

fn default_asset_id() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "jar-only"
    }
}

fn default_cache_dir() -> PathBuf {
    if let Some(value) = env::var_os("SOURCEWEAVER_TOOL_CACHE") {
        return PathBuf::from(value);
    }
    if cfg!(target_os = "windows") {
        if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
            return PathBuf::from(local_app_data)
                .join("SourceWeaver")
                .join("tools");
        }
    } else if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".cache")
            .join("sourceweaver")
            .join("tools");
    }
    PathBuf::from(".sourceweaver-tools")
}

fn cache_file_path(cache_dir: &Path, asset: &BspSourceManagedAsset) -> PathBuf {
    cache_dir
        .join("bspsource")
        .join(asset.version)
        .join(asset.name)
}

fn sha256_file(path: &Path) -> Result<(u64, String), String> {
    let mut file = fs::File::open(path)
        .map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        size += read as u64;
        hasher.update(&buffer[..read]);
    }
    Ok((size, format!("{:x}", hasher.finalize())))
}

fn normalize_sha256(value: &str) -> Result<String, String> {
    let value = value
        .trim()
        .strip_prefix("sha256:")
        .unwrap_or(value.trim())
        .to_ascii_lowercase();
    if value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Ok(value)
    } else {
        Err(format!("invalid sha256 digest `{value}`"))
    }
}

fn asset_json(asset: &BspSourceManagedAsset) -> serde_json::Value {
    serde_json::json!({
        "id": asset.id,
        "name": asset.name,
        "platform": asset.platform,
        "version": asset.version,
        "url": asset.url,
        "sha256": asset.sha256,
        "size": asset.size,
        "release_url": asset.release_url,
        "source_tag": asset.source_tag,
        "java_note": asset.java_note,
    })
}

fn manifest_json() -> serde_json::Value {
    serde_json::json!({
        "ok": true,
        "tool": "BSPSource",
        "version": BSPSOURCE_MANAGED_VERSION,
        "release_url": BSPSOURCE_RELEASE_URL,
        "source_tag": BSPSOURCE_SOURCE_TAG,
        "license_url": BSPSOURCE_LICENSE_URL,
        "license_review": {
            "bspsource": "Unlicense per upstream LICENSE.md fetched from ata4/bspsrc master during issue #100 research.",
            "dependencies": [
                "Apache Log4j 2, Apache Commons Compress, picocli, FlatLaf, and jSystemThemeDetector listed as Apache-2.0 by upstream LICENSE.md.",
                "MigLayout listed as BSD-3-Clause by upstream LICENSE.md."
            ]
        },
        "assets": BSPSOURCE_ASSETS.iter().map(asset_json).collect::<Vec<_>>(),
    })
}

fn policy_json() -> serde_json::Value {
    serde_json::json!({
        "ok": true,
        "redistribution_decision": "do-not-bundle; user-initiated download/cache only",
        "local_paths_supported": true,
        "version_pin": BSPSOURCE_MANAGED_VERSION,
        "provenance": {
            "release_api": "https://api.github.com/repos/ata4/bspsrc/releases",
            "release_url": BSPSOURCE_RELEASE_URL,
            "source_tag": BSPSOURCE_SOURCE_TAG,
            "license_url": BSPSOURCE_LICENSE_URL
        },
        "checksum_policy": "Every managed asset is pinned by SHA-256 digest and expected size from the GitHub release API. Downloads are written to a partial file, verified, then moved into cache.",
        "cache_policy": "Cache path defaults to SOURCEWEAVER_TOOL_CACHE when set, otherwise a per-user Source Weaver tools cache. Cache entries are versioned by tool/version/asset name.",
        "update_policy": "Updates require a Source Weaver manifest change with new version, release URL, asset URL, size, and SHA-256 digest. Automatic latest-version adoption is intentionally not implemented.",
        "download_policy": "Downloads require explicit --accept-download-policy. Source Weaver does not bundle BSPSource assets in releases.",
        "execution_policy": "Managed download only obtains and verifies the upstream ZIP. It does not run BSPSource, extract game content, validate maps, or imply decompile quality.",
        "alternatives": ["Existing user-selected BSPSource launcher path", "Existing user-selected BSPSource jar path", "Existing generic wrapper path"]
    })
}

fn print_help() {
    println!("sourceweaver bspsource manifest [--json]");
    println!("sourceweaver bspsource policy [--json]");
    println!(
        "sourceweaver bspsource cache-path [--asset jar-only|linux|windows] [--cache-dir dir] [--json]"
    );
    println!(
        "sourceweaver bspsource verify --file zip [--asset jar-only|linux|windows | --sha256 hex] [--json]"
    );
    println!(
        "sourceweaver bspsource download [--asset jar-only|linux|windows] [--cache-dir dir] --accept-download-policy [--json]"
    );
    println!();
    println!(
        "Managed BSPSource downloads are user-initiated, version-pinned, checksum-verified, and cached. Source Weaver does not bundle BSPSource; local tool paths remain supported."
    );
}

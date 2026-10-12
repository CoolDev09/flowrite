fn main() {
    license_report();
    println!("cargo:rerun-if-changed=Cargo.lock");
    println!("cargo:rerun-if-env-changed=FLOWRITE_FETCH_GRAMMAR");
    println!("cargo:rerun-if-changed=assets/nlprule/en_rules.bin");
    println!("cargo:rerun-if-changed=assets/nlprule/en_tokenizer.bin");
    if std::env::var_os("FLOWRITE_FETCH_GRAMMAR").is_some() {
        std::fs::create_dir_all("assets/nlprule").expect("create grammar asset folder");
        nlprule_build::BinaryBuilder::new(&["en"], "assets/nlprule")
            .version("0.6.4")
            .cache_dir(Some(std::path::PathBuf::from("../../work/grammar-cache")))
            .fallback_to_build_dir(false)
            .build()
            .expect("download pinned English grammar data")
            .validate()
            .expect("validate English grammar data");
    }
}

fn license_report() {
    use std::path::PathBuf;
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                std::env::var_os("USERPROFILE")
                    .or_else(|| std::env::var_os("HOME"))
                    .unwrap_or_default(),
            )
            .join(".cargo")
        });
    let registries: Vec<_> = std::fs::read_dir(cargo_home.join("registry/src"))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    let lock = std::fs::read_to_string("Cargo.lock").expect("read Cargo.lock");
    let mut report=String::from("flowrite third-party dependency notices\nGenerated from Cargo.lock and the downloaded crate manifests at build time.\nIncludes target-specific and build-time dependencies; not all are linked into the executable.\n\n");
    for package in lock.split("[[package]]").skip(1) {
        let field = |key: &str| {
            package
                .lines()
                .find_map(|line| line.strip_prefix(&format!("{key} = \"")))
                .and_then(|value| value.split('"').next())
        };
        let (Some(name), Some(version)) = (field("name"), field("version")) else {
            continue;
        };
        if name == "flowrite" {
            continue;
        }
        report.push_str(&format!("\n{name} {version}\n"));
        let source = registries
            .iter()
            .map(|r| r.join(format!("{name}-{version}")))
            .find(|p| p.join("Cargo.toml").exists());
        if let Some(source) = source {
            let manifest = std::fs::read_to_string(source.join("Cargo.toml")).unwrap_or_default();
            for key in ["license", "license-file", "repository"] {
                if let Some(value) = manifest
                    .lines()
                    .find_map(|line| line.strip_prefix(&format!("{key} = ")))
                {
                    report.push_str(&format!("{key}: {value}\n"));
                }
            }
            let mut notices: Vec<_> = std::fs::read_dir(&source)
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().to_ascii_uppercase();
                    name.starts_with("LICENSE")
                        || name.starts_with("COPYING")
                        || name.starts_with("NOTICE")
                })
                .collect();
            notices.sort_by_key(|e| e.file_name());
            for notice in notices {
                if let Ok(text) = std::fs::read_to_string(notice.path()) {
                    report.push_str(&format!(
                        "\n{}\n{text}\n",
                        notice.file_name().to_string_lossy()
                    ));
                }
            }
        } else {
            report.push_str(
                "Source/notice not present for this target; see the versioned package source at ",
            );
            report.push_str(&format!("https://crates.io/crates/{name}/{version}\n"));
        }
    }
    let path = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"))
        .join("third-party-licenses.txt");
    std::fs::write(path, report).expect("write third-party notices");
}

//! Evidence-based technology detection (spec section 7).
//!
//! Every detection here is justified by a concrete file or manifest field —
//! never a guess. If we can't find evidence for something, we don't report
//! it. Each `TechDetection` records what the evidence was so the UI (and
//! the user) can see *why* Haleacas thinks a repo uses a given technology.

use crate::types::TechDetection;
use std::fs;
use std::path::Path;

pub fn detect(root: &Path) -> Vec<TechDetection> {
    let mut techs = Vec::new();

    detect_node(root, &mut techs);
    detect_rust(root, &mut techs);
    detect_python(root, &mut techs);
    detect_go(root, &mut techs);
    detect_dart_flutter(root, &mut techs);
    detect_ruby(root, &mut techs);
    detect_jvm(root, &mut techs);

    techs
}

fn push(techs: &mut Vec<TechDetection>, name: &str, evidence: &str) {
    if !techs.iter().any(|t| t.name == name) {
        techs.push(TechDetection {
            name: name.to_string(),
            evidence: evidence.to_string(),
        });
    }
}

fn detect_node(root: &Path, techs: &mut Vec<TechDetection>) {
    let pkg_path = root.join("package.json");
    let Ok(raw) = fs::read_to_string(&pkg_path) else {
        return;
    };
    push(techs, "Node.js", "package.json present");

    let Ok(pkg): Result<serde_json::Value, _> = serde_json::from_str(&raw) else {
        return;
    };

    let mut deps: Vec<String> = Vec::new();
    for field in ["dependencies", "devDependencies", "peerDependencies"] {
        if let Some(obj) = pkg.get(field).and_then(|v| v.as_object()) {
            deps.extend(obj.keys().cloned());
        }
    }
    let has = |name: &str| deps.iter().any(|d| d == name);

    if has("typescript") || root.join("tsconfig.json").is_file() {
        push(techs, "TypeScript", "typescript dependency or tsconfig.json");
    }
    if has("react") {
        push(techs, "React", "react in package.json dependencies");
    }
    if has("vue") {
        push(techs, "Vue", "vue in package.json dependencies");
    }
    if has("svelte") {
        push(techs, "Svelte", "svelte in package.json dependencies");
    }
    if has("@angular/core") {
        push(techs, "Angular", "@angular/core in package.json dependencies");
    }
    if has("@tauri-apps/api") || root.join("src-tauri").is_dir() {
        push(techs, "Tauri", "@tauri-apps/api dependency or src-tauri/ directory");
    }
    if has("vite") {
        push(techs, "Vite", "vite in package.json dependencies");
    }
    if has("next") {
        push(techs, "Next.js", "next in package.json dependencies");
    }
    if fs::metadata(root.join("pnpm-lock.yaml")).is_ok() {
        push(techs, "pnpm", "pnpm-lock.yaml present");
    } else if fs::metadata(root.join("yarn.lock")).is_ok() {
        push(techs, "Yarn", "yarn.lock present");
    } else if fs::metadata(root.join("package-lock.json")).is_ok() {
        push(techs, "npm", "package-lock.json present");
    }
}

fn detect_rust(root: &Path, techs: &mut Vec<TechDetection>) {
    let cargo_path = root.join("Cargo.toml");
    let Ok(raw) = fs::read_to_string(&cargo_path) else {
        return;
    };
    push(techs, "Rust", "Cargo.toml present");

    if let Ok(doc) = raw.parse::<toml::Value>() {
        let has_dep = |name: &str| -> bool {
            for section in ["dependencies", "build-dependencies", "dev-dependencies"] {
                if doc.get(section).and_then(|d| d.get(name)).is_some() {
                    return true;
                }
            }
            false
        };
        if has_dep("tauri") {
            push(techs, "Tauri", "tauri in Cargo.toml dependencies");
        }
        if has_dep("actix-web") {
            push(techs, "Actix Web", "actix-web in Cargo.toml dependencies");
        }
        if has_dep("axum") {
            push(techs, "Axum", "axum in Cargo.toml dependencies");
        }
        if has_dep("tokio") {
            push(techs, "Tokio", "tokio in Cargo.toml dependencies");
        }
    }
}

fn detect_python(root: &Path, techs: &mut Vec<TechDetection>) {
    let has_pyproject = root.join("pyproject.toml").is_file();
    let has_requirements = root.join("requirements.txt").is_file();
    let has_setup = root.join("setup.py").is_file();
    if !(has_pyproject || has_requirements || has_setup) {
        return;
    }
    let evidence = if has_pyproject {
        "pyproject.toml present"
    } else if has_requirements {
        "requirements.txt present"
    } else {
        "setup.py present"
    };
    push(techs, "Python", evidence);

    if let Ok(raw) = fs::read_to_string(root.join("requirements.txt")) {
        if raw.lines().any(|l| l.trim_start().starts_with("django")) {
            push(techs, "Django", "django in requirements.txt");
        }
        if raw.lines().any(|l| l.trim_start().starts_with("flask")) {
            push(techs, "Flask", "flask in requirements.txt");
        }
        if raw.lines().any(|l| l.trim_start().starts_with("fastapi")) {
            push(techs, "FastAPI", "fastapi in requirements.txt");
        }
    }
}

fn detect_go(root: &Path, techs: &mut Vec<TechDetection>) {
    if root.join("go.mod").is_file() {
        push(techs, "Go", "go.mod present");
    }
}

fn detect_dart_flutter(root: &Path, techs: &mut Vec<TechDetection>) {
    let Ok(raw) = fs::read_to_string(root.join("pubspec.yaml")) else {
        return;
    };
    push(techs, "Dart", "pubspec.yaml present");
    if raw.contains("flutter:") || raw.contains("sdk: flutter") {
        push(techs, "Flutter", "flutter SDK reference in pubspec.yaml");
    }
}

fn detect_ruby(root: &Path, techs: &mut Vec<TechDetection>) {
    if root.join("Gemfile").is_file() {
        push(techs, "Ruby", "Gemfile present");
    }
}

fn detect_jvm(root: &Path, techs: &mut Vec<TechDetection>) {
    if root.join("pom.xml").is_file() {
        push(techs, "Java", "pom.xml present (Maven)");
    }
    if root.join("build.gradle").is_file() || root.join("build.gradle.kts").is_file() {
        push(techs, "Gradle", "build.gradle(.kts) present");
        if root.join("build.gradle.kts").is_file() {
            push(techs, "Kotlin", "build.gradle.kts present");
        }
    }
}

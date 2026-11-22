//! Maps the configured editor + a file path (and optional line number) to
//! a concrete `(binary, args)` pair — pure logic, no process spawning
//! here, so it's testable without actually launching an editor.
//!
//! `Settings.editor` holds a plain string: one of a few recognized keys
//! ("vscode", "cursor", "zed", "sublime") for editors with a known
//! line-jump convention, or — for anything else — the string is treated
//! as a custom executable name/path directly (spec section 25: "custom
//! executable"). No new settings field was needed for this.
//!
//! Args are always built as a real argument list, never a shell string —
//! whatever a filename contains (spaces, quotes, anything) is passed to
//! the process exactly as one argument, with no shell involved to
//! misinterpret it (spec section 35: sanitized paths, no arbitrary shell
//! execution).

pub fn launch_args(editor_setting: &str, path: &str, line: Option<u32>) -> (String, Vec<String>) {
    match editor_setting {
        "vscode" => vscode_like("code", path, line),
        "cursor" => vscode_like("cursor", path, line),
        "zed" => (
            "zed".to_string(),
            vec![match line {
                Some(l) => format!("{path}:{l}"),
                None => path.to_string(),
            }],
        ),
        "sublime" => (
            "subl".to_string(),
            vec![match line {
                Some(l) => format!("{path}:{l}"),
                None => path.to_string(),
            }],
        ),
        // Anything unrecognized is treated as the user's own custom
        // executable. We can't know its line-jump convention, so only
        // the bare path is passed — best-effort, documented as such.
        other => (other.to_string(), vec![path.to_string()]),
    }
}

fn vscode_like(binary: &str, path: &str, line: Option<u32>) -> (String, Vec<String>) {
    match line {
        Some(l) => (binary.to_string(), vec!["-g".to_string(), format!("{path}:{l}")]),
        None => (binary.to_string(), vec![path.to_string()]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vscode_uses_goto_flag_with_line() {
        let (bin, args) = launch_args("vscode", "/repo/src/a.ts", Some(42));
        assert_eq!(bin, "code");
        assert_eq!(args, vec!["-g", "/repo/src/a.ts:42"]);
    }

    #[test]
    fn vscode_without_line_just_opens_the_path() {
        let (bin, args) = launch_args("vscode", "/repo/src/a.ts", None);
        assert_eq!(bin, "code");
        assert_eq!(args, vec!["/repo/src/a.ts"]);
    }

    #[test]
    fn cursor_uses_the_same_convention_as_vscode() {
        let (bin, args) = launch_args("cursor", "/repo/a.ts", Some(1));
        assert_eq!(bin, "cursor");
        assert_eq!(args, vec!["-g", "/repo/a.ts:1"]);
    }

    #[test]
    fn zed_embeds_the_line_directly_in_the_path_argument() {
        let (bin, args) = launch_args("zed", "/repo/a.ts", Some(10));
        assert_eq!(bin, "zed");
        assert_eq!(args, vec!["/repo/a.ts:10"]);
    }

    #[test]
    fn sublime_embeds_the_line_directly_in_the_path_argument() {
        let (bin, args) = launch_args("sublime", "/repo/a.ts", Some(10));
        assert_eq!(bin, "subl");
        assert_eq!(args, vec!["/repo/a.ts:10"]);
    }

    #[test]
    fn unrecognized_key_is_treated_as_a_custom_executable() {
        let (bin, args) = launch_args("/usr/local/bin/my-editor", "/repo/a.ts", Some(5));
        assert_eq!(bin, "/usr/local/bin/my-editor");
        // No known line-jump convention for a custom command — just the
        // bare path, not a guessed flag.
        assert_eq!(args, vec!["/repo/a.ts"]);
    }

    #[test]
    fn path_with_spaces_stays_a_single_argument() {
        let (_, args) = launch_args("vscode", "/repo/my folder/a.ts", None);
        assert_eq!(args, vec!["/repo/my folder/a.ts"]);
    }
}

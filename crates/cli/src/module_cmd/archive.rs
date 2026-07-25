//! Archive path detection for `module install`.

use std::path::Path;

pub(super) fn is_archive(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    // `name` already lowercased; compound `.tar.gz` is not a single extension.
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    {
        name.ends_with(".tar.gz")
            || name.ends_with(".tgz")
            || path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("tar"))
    }
}

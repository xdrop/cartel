use std::path::{Path, PathBuf};

/// Expands the `~` symbol in paths.
///
/// Replaces the tilde in a path with the path to the users home directory.
pub fn expand_tilde<P: AsRef<Path>>(path: P) -> Option<PathBuf> {
    let p = path.as_ref();
    if !p.starts_with("~") {
        return Some(p.to_path_buf());
    }
    if p == Path::new("~") {
        return dirs::home_dir();
    }
    dirs::home_dir().map(|mut h| {
        if h == Path::new("/") {
            p.strip_prefix("~").unwrap().to_path_buf()
        } else {
            h.push(p.strip_prefix("~/").unwrap());
            h
        }
    })
}

/// Construct a [PathBuf] from the given `&str`.
///
/// Convert the given path string onto a [PathBuf] expanding all tilde to the
/// users directory.
pub fn from_user_str(path: &str) -> Option<PathBuf> {
    expand_tilde(PathBuf::from(path))
}

/// Construct a [PathBuf] from the given `String`.
///
/// Convert the given path string onto a [PathBuf] expanding all tilde to the
/// users directory.
pub fn from_user_string(path: String) -> Option<PathBuf> {
    expand_tilde(PathBuf::from(path))
}
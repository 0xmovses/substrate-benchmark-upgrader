#[cfg(test)]
mod tests {
    use super::super::*;
use std::fs;
    use std::path::Path;

    #[test]
    fn test_upgrade_benchmark() {
        let fixtures_dir = Path::new("tests/fixtures");
        let dummy_path = fixtures_dir.join("dummy.rs");
        let dummy_code = fs::read_to_string(dummy_path)
            .expect("Failed to read dummy.rs file");

        // parse and apply your macro as before
    }
}

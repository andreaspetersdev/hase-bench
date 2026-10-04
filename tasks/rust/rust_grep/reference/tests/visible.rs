use std::io::Write;
use std::process::{Command, Stdio};

fn search(args: &[&str], input: &[u8]) -> (i32, Vec<u8>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rust_grep"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start rust_grep");
    let _ = child.stdin.take().unwrap().write_all(input);
    let output = child.wait_with_output().unwrap();
    (output.status.code().unwrap(), output.stdout)
}

#[test]
fn fixed_match_preserves_bytes_and_uses_lf() {
    assert_eq!(
        search(&["-F", "alpha"], b"alpha\r\nbeta\nalpha tail"),
        (0, b"alpha\nalpha tail\n".to_vec())
    );
}

#[test]
fn ere_match_and_line_number() {
    assert_eq!(
        search(&["-n", "^b(eta|ravo)$"], b"alpha\nbeta\nbravo\n"),
        (0, b"2:beta\n3:bravo\n".to_vec())
    );
}

#[test]
fn no_match_has_status_one() {
    assert_eq!(search(&["-F", "missing"], b"alpha\n"), (1, Vec::new()));
}

#[test]
fn patterns_from_arguments_and_files() {
    assert_eq!(
        search(&["-F", "-e", "missing", "-e", "beta"], b"alpha\nbeta\n"),
        (0, b"beta\n".to_vec())
    );
    assert_eq!(
        search(&["-F", "-e", ""], b"alpha\n\nbeta"),
        (0, b"alpha\n\nbeta\n".to_vec())
    );
    let path = std::env::temp_dir().join(format!("rust-grep-patterns-{}", std::process::id()));
    std::fs::write(&path, b"missing\nbeta").unwrap();
    let path_text = path.to_str().unwrap();
    assert_eq!(
        search(&["-F", "-f", path_text], b"alpha\nbeta\n"),
        (0, b"beta\n".to_vec())
    );
    std::fs::write(&path, b"").unwrap();
    assert_eq!(
        search(&["-F", "-f", path_text], b"alpha\nbeta\n"),
        (1, Vec::new())
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn fixed_ascii_boundaries() {
    assert_eq!(
        search(&["-F", "-i", "-w", "CAT"], b"cat\nscatter\ncat-\n"),
        (0, b"cat\ncat-\n".to_vec())
    );
    assert_eq!(
        search(&["-F", "-x", "cat"], b"cat\ncat-\n"),
        (0, b"cat\n".to_vec())
    );
}

#[test]
fn binary_modes() {
    assert_eq!(
        search(&["-F", "needle"], b"\0needle\n"),
        (0, b"Binary file (standard input) matches\n".to_vec())
    );
    assert_eq!(
        search(&["-F", "-c", "needle"], b"\0needle\n"),
        (0, b"1\n".to_vec())
    );
}

#[test]
fn long_line() {
    let exact = vec![b'a'; 16 * 1024 * 1024];
    assert_eq!(search(&["-F", "-q", "a"], &exact), (0, Vec::new()));
    let mut excess = exact;
    excess.push(b'a');
    assert_eq!(search(&["-F", "-q", "a"], &excess), (2, Vec::new()));
}

#[test]
fn selection_modes() {
    assert_eq!(
        search(&["-F", "-v", "-c", "hit"], b"hit\nmiss\nmiss\n"),
        (0, b"2\n".to_vec())
    );
    assert_eq!(
        search(&["-F", "-L", "hit"], b"miss\n"),
        (0, b"(standard input)\n".to_vec())
    );
    assert_eq!(
        search(&["-F", "-q", "-c", "hit"], b"hit\n"),
        (0, Vec::new())
    );
}

#[test]
fn presentation() {
    assert_eq!(
        search(&["-F", "-o", "ana"], b"banana\n"),
        (0, b"ana\n".to_vec())
    );
    assert_eq!(
        search(
            &["-F", "-n", "-B1", "-A1", "hit"],
            b"a\nhit\nb\nc\nd\nhit\ne\n"
        ),
        (0, b"1-a\n2:hit\n3-b\n--\n5-d\n6:hit\n7-e\n".to_vec())
    );
}

#[test]
fn recursive_tree() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("rust-grep-tree-{}-{unique}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let first = root.join("a.txt");
    let second = root.join("b.log");
    std::fs::write(&first, b"hit\n").unwrap();
    std::fs::write(&second, b"hit\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rust_grep"))
        .args(["-F", "-r", "--include=*.txt", "hit", root.to_str().unwrap()])
        .output()
        .unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        output.stdout,
        format!("{}:hit\n", first.display()).into_bytes()
    );
}

#[test]
fn exit_status() {
    let missing = std::env::temp_dir().join(format!(
        "rust-grep-missing-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = missing.to_str().unwrap();
    assert_eq!(
        search(&["-F", "hit", path, "-"], b"hit\n"),
        (2, b"(standard input):hit\n".to_vec())
    );
    assert_eq!(
        search(&["-F", "-q", "hit", "-", path], b"hit\n"),
        (0, Vec::new())
    );
}

#[test]
fn cli_pattern_sources() {
    assert_eq!(
        search(&["-F", "--", "-dash"], b"-dash\nother\n"),
        (0, b"-dash\n".to_vec())
    );
    assert_eq!(
        search(&["-F", "-e", "other"], b"-dash\nother\n"),
        (0, b"other\n".to_vec())
    );
}

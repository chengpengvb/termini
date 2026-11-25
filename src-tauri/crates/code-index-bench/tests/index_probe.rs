use std::fs;
use std::time::Instant;
use code_index_bench::module::ai::code_index::walker::probe_index_size;

fn make_tree(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join("termini-probe-test").join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create root");
    root
}

fn write_rs(dir: &std::path::Path, name: &str, kib: usize) {
    fs::create_dir_all(dir).expect("create dir");
    let body = "// x\nfn f() {}\n".repeat(kib * 1024 / 12);
    fs::write(dir.join(name), body).expect("write file");
}

#[test]
fn skips_build_artifacts() {
    let root = make_tree("artifacts");
    write_rs(&root.join("src"), "lib.rs", 8);
    write_rs(&root.join("target/debug"), "huge.rs", 512);
    write_rs(&root.join("node_modules/pkg"), "index.js", 512);
    write_rs(&root.join("CMakeFiles"), "gen.c", 512);

    let probe = probe_index_size(&root, &[], false, 64 * 1024 * 1024).expect("probe");
    assert!(!probe.exceeded, "不该超限");
    assert_eq!(probe.files, 1, "只应统计 src/lib.rs, files={}", probe.files);
    assert!(probe.bytes < 64 * 1024, "体量应只有源码那点, bytes={}", probe.bytes);
}

#[test]
fn early_stops_when_over_limit() {
    let root = make_tree("oversize");
    for index in 0..40 {
        write_rs(&root.join(format!("mod{}", index)), "code.rs", 256);
    }
    let started = Instant::now();
    let probe = probe_index_size(&root, &[], false, 1024 * 1024).expect("probe");
    let elapsed = started.elapsed();

    assert!(probe.exceeded, "应判定超限");
    assert!(probe.bytes < 4 * 1024 * 1024, "应在刚过上限时就中止, bytes={}", probe.bytes);
    assert!(elapsed.as_secs() < 5, "探测应是秒级, elapsed={:?}", elapsed);
}

#[test]
fn counts_only_indexable_extensions() {
    let root = make_tree("exts");
    write_rs(&root.join("src"), "a.rs", 4);
    fs::write(root.join("src/blob.bin"), vec![0u8; 512 * 1024]).expect("write bin");
    fs::write(root.join("src/image.png"), vec![0u8; 512 * 1024]).expect("write png");

    let probe = probe_index_size(&root, &[], false, 64 * 1024 * 1024).expect("probe");
    assert_eq!(probe.files, 1, "只应统计 a.rs, files={}", probe.files);
}

#[test]
fn single_file_root_is_supported() {
    let root = make_tree("single");
    write_rs(&root, "one.rs", 4);
    let probe = probe_index_size(&root.join("one.rs"), &[], false, 64 * 1024 * 1024).expect("probe");
    assert_eq!(probe.files, 1);
    assert!(probe.bytes > 0);
}

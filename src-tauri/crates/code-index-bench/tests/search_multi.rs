use std::fs;
use std::path::{Path, PathBuf};
use code_index_bench::module::ai::code_index::render::render_search_grouped;
use code_index_bench::module::ai::code_index::walker::{search_files_multi, SearchHit};

fn fixture() -> PathBuf {
    static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("termini-search-multi-{}-{}", std::process::id(), seq));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).expect("create fixture");
    fs::write(dir.join("src/alpha.rs"), "fn alpha() {\n    let token = 1;\n    println!(\"alpha\");\n}\n").expect("write alpha");
    fs::write(dir.join("src/beta.rs"), "fn beta() {\n    let budget = 2;\n    println!(\"beta token\");\n}\n").expect("write beta");
    fs::write(dir.join("src/gamma.rs"), "fn gamma() {\n    // nothing interesting here\n}\n").expect("write gamma");
    dir
}

fn queries(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

#[test]
fn groups_results_per_query_in_order() {
    let root = fixture();
    let list = queries(&["token", "budget", "nonexistent_zzz"]);
    let grouped = search_files_multi(&root, &list, None, None).expect("search");

    assert_eq!(grouped.len(), 3, "分组数必须与关键字数一致");
    // token 在 alpha.rs 和 beta.rs 各一处
    assert_eq!(grouped[0].len(), 2, "token 应命中 2 处, 实际 {:?}", grouped[0]);
    assert_eq!(grouped[1].len(), 1, "budget 应命中 1 处");
    assert!(grouped[2].is_empty(), "不存在的关键字必须是空组, 而不是串到别的组里");
    assert!(grouped[1][0].path.ends_with("beta.rs"));
}

#[test]
fn per_query_match_limit_is_independent() {
    let root = fixture();
    let list = queries(&["token", "budget"]);
    let grouped = search_files_multi(&root, &list, None, Some(1)).expect("search");
    // 额度是每个关键字各自的, 不是所有关键字共享一个总数
    assert_eq!(grouped[0].len(), 1);
    assert_eq!(grouped[1].len(), 1);
}

#[test]
fn glob_filter_applies_to_every_query() {
    let root = fixture();
    let list = queries(&["token", "budget"]);
    let grouped = search_files_multi(&root, &list, Some("**/beta.rs"), None).expect("search");
    assert_eq!(grouped[0].len(), 1, "token 在 beta.rs 里只有一处");
    assert!(grouped[0][0].path.ends_with("beta.rs"));
    assert_eq!(grouped[1].len(), 1);
}

#[test]
fn line_anchors_match_every_line_not_just_file_start() {
    let root = fixture();
    fs::write(root.join("src/imports.java"), "package a;\r\nimport com.yzfar.monitor.util.FTPUtil;\r\nimport java.util.List;\r\npublic class X {}\r\n").expect("write");
    let grouped = search_files_multi(&root, &queries(&[r"^import com\.yzfar", r"List;$", r"^public class"]), Some("**/imports.java"), None).expect("search");
    assert_eq!(grouped[0].len(), 1, "^ 必须匹配第 2 行的行首: {:?}", grouped[0]);
    assert_eq!(grouped[0][0].line, 2);
    assert_eq!(grouped[1].len(), 1, "$ 必须匹配行尾");
    assert_eq!(grouped[2][0].line, 4);
}

#[test]
fn brace_glob_selects_several_subtrees() {
    let root = fixture();
    fs::create_dir_all(root.join("other")).expect("mkdir");
    fs::write(root.join("other/delta.rs"), "fn delta() { let token = 9; }\n").expect("write delta");
    let grouped = search_files_multi(&root, &queries(&["token"]), Some("{src,nowhere}/**"), None).expect("search");
    assert_eq!(grouped[0].len(), 2, "只应命中 src 下的两处, 不应带上 other/delta.rs: {:?}", grouped[0]);
    assert!(grouped[0].iter().all(|hit| !hit.path.ends_with("delta.rs")));
}

#[test]
fn rejects_empty_and_invalid_regex() {
    let root = fixture();
    assert!(search_files_multi(&root, &[], None, None).is_err(), "空关键字列表必须报错");
    assert!(search_files_multi(&root, &queries(&["token", "   "]), None, None).is_err(), "空白关键字必须报错");
    assert!(search_files_multi(&root, &queries(&["("]), None, None).is_err(), "非法正则必须报错");
}

fn hit(path: &str, line: usize, text: &str) -> SearchHit {
    SearchHit { path: PathBuf::from(path), line, text: text.to_string(), ..Default::default() }
}

#[test]
fn hits_carry_surrounding_context_lines() {
    let root = fixture();
    let grouped = search_files_multi(&root, &queries(&["fn beta"]), None, None).expect("search ok");
    let hit = grouped[0].first().expect("fixture 里必须有 fn beta");
    assert!(!hit.before.is_empty() || !hit.after.is_empty(), "命中必须带前后上下文, 否则模型下一轮又要 file_read 一趟");

    let out = render_search_grouped(&root, &queries(&["fn beta"]), &grouped, 10000);
    let match_pos = out.find(&format!(":{}:fn beta", hit.line)).expect("命中行");
    for (index, line) in hit.after.iter().enumerate() {
        let rendered = format!("  {:>5}  {}", hit.line + 1 + index, line.trim_end());
        let pos = out.find(&rendered).unwrap_or_else(|| panic!("上下文行缺失: {rendered}\n{out}"));
        assert!(pos > match_pos, "后置上下文必须排在命中行之后");
    }
}

#[test]
fn rendered_output_is_grouped_by_query() {
    let list = queries(&["alpha", "beta"]);
    let grouped = vec![vec![hit("/repo/a.rs", 3, "let alpha = 1;")], vec![]];
    let out = render_search_grouped(Path::new("/repo"), &list, &grouped, 10000);

    assert!(out.contains("## alpha (1 matches in 1 files)"), "缺少 alpha 分组头: {out}");
    assert!(out.contains("a.rs:3:let alpha = 1;"), "缺少命中行: {out}");
    assert!(out.contains("## beta\n(no matches)"), "无命中的关键字也要出现在结果里: {out}");
    assert!(out.find("## alpha").unwrap() < out.find("## beta").unwrap(), "分组顺序必须与关键字顺序一致");
}

#[test]
fn oversized_group_keeps_head_and_tail() {
    let list = queries(&["wide"]);
    let hits: Vec<SearchHit> = (1..=400).map(|line| hit("/repo/big.rs", line, &format!("marker_{line} some padding text here to burn tokens"))).collect();
    let out = render_search_grouped(Path::new("/repo"), &list, &[hits], 300);

    assert!(out.contains("tokens omitted"), "超预算必须给出省略提示: {out}");
    assert!(out.contains("marker_1 "), "必须保留头部: {out}");
    assert!(out.contains("marker_400"), "必须保留尾部, 而不是从中间截断丢掉后半段: {out}");
}

#[test]
fn budget_left_over_by_empty_groups_flows_to_later_ones() {
    // 前两个关键字没有命中, 第三个很大: 省下的额度应该流给它, 使它比均分时保留更多
    let list = queries(&["none1", "none2", "wide"]);
    let hits: Vec<SearchHit> = (1..=400).map(|line| hit("/repo/big.rs", line, &format!("marker_{line} some padding text here to burn tokens"))).collect();
    let grouped = vec![vec![], vec![], hits.clone()];
    let shared = render_search_grouped(Path::new("/repo"), &list, &grouped, 900);
    let alone = render_search_grouped(Path::new("/repo"), &queries(&["wide"]), &[hits], 300);

    let count = |text: &str| text.matches("marker_").count();
    assert!(count(&shared) > count(&alone), "空分组省下的额度应流给后面的分组: shared={} alone={}", count(&shared), count(&alone));
}

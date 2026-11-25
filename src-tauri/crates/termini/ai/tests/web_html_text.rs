use termini_ai::module::ai::tools::common::web::html_text::{html_to_text, looks_like_html};

const PAGE: &str = r#"<!DOCTYPE html>
<html lang="zh-Hans"><head><meta charset="UTF-8"><title>鸿禹智慧运维平台V2.0 - 重庆市软件正版化服务平台</title>
<style>.a{color:red}</style><script>window.x = "<p>not text</p>";</script></head>
<body><!-- comment --><nav><a href="/">首页</a> &gt; <a href="/p">产品</a></nav>
<div class="main"><h1>鸿禹智慧运维平台 V2.0</h1>
<p>定位&ldquo;自主可控的全国产化运维平台&rdquo;，监测对象覆盖龙芯/飞腾&nbsp;服务器。</p>
<ul><li>软件著作权登记</li><li>版本&#x56;2.0</li></ul>
<table><tr><td>厂商</td><td>重庆远臻科技有限公司</td></tr></table>
</div><footer>&copy; 2025</footer><script src="app.js"></script></body></html>"#;

#[test]
fn detects_html_documents() {
    assert!(looks_like_html(PAGE));
    assert!(looks_like_html("  <html><body>x</body></html>"));
    assert!(!looks_like_html("# markdown\nplain text"));
    assert!(!looks_like_html("{\"json\": true}"));
}

#[test]
fn strips_tags_scripts_and_styles() {
    let text = html_to_text(PAGE);
    assert!(!text.contains("<"), "tags left: {text}");
    assert!(!text.contains("color:red"));
    assert!(!text.contains("not text"));
    assert!(!text.contains("comment"));
    assert!(text.starts_with("# 鸿禹智慧运维平台V2.0 - 重庆市软件正版化服务平台"));
}

#[test]
fn keeps_body_text_with_line_breaks_and_entities() {
    let text = html_to_text(PAGE);
    assert!(text.contains("首页 > 产品"));
    assert!(text.contains("定位“自主可控的全国产化运维平台”，监测对象覆盖龙芯/飞腾 服务器。"));
    assert!(text.contains("软件著作权登记\n版本V2.0"));
    assert!(text.contains("厂商\n重庆远臻科技有限公司"));
    assert!(text.contains("© 2025"));
    assert!(!text.contains("\n\n"));
}

#[test]
fn shrinks_markup_heavy_pages() {
    let text = html_to_text(PAGE);
    assert!(text.chars().count() * 2 < PAGE.chars().count(), "text={} page={}", text.chars().count(), PAGE.chars().count());
}

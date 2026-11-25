use termini_ai::module::ai::tools::common::task::rule::shell_readonly::{absolute_paths, check_readonly};

#[test]
fn plain_readonly_commands_pass() {
    for command in [
        "rg -n 'class Foo' src",
        "Get-ChildItem -Recurse -Filter *.xml | Select-Object FullName,Length",
        "git log --oneline -20 -- center/src",
        "git blame -L 10,30 src/A.java",
        "mvn dependency:tree -Dscope=compile",
        "find . -name '*.properties' | head -50",
        "cat pom.xml | grep -n artifactId",
        "wc -l $(rg -l 'Sharding' src)",
        "$files = Get-ChildItem -Recurse *.java; $files | Measure-Object",
        "ls -la 2>&1",
        "rg --files . | Select-Object -First 40; \"--- count ---\"; (rg --files . | Measure-Object).Count",
        "Get-ChildItem -Recurse -File | ForEach-Object { \"$($_.FullName) [$([math]::Round($_.Length/1KB,1)) KB]\" }",
    ] {
        assert!(check_readonly(command).is_ok(), "应放行: {command} -> {:?}", check_readonly(command));
    }
}

#[test]
fn writes_and_dangerous_forms_are_rejected() {
    for command in [
        "echo hi > out.txt",
        "rg foo | tee log.txt",
        "Get-Content a | Out-File b",
        "Remove-Item -Recurse target",
        "rm -rf build",
        "git checkout -- .",
        "git branch -d feature",
        "git config user.name x",
        "mvn clean install",
        "npm install",
        "sed -i 's/a/b/' file",
        "find . -name '*.log' | xargs rm",
        "curl https://example.com",
        "sudo ls",
        "Start-Process notepad",
        "cd /tmp && ls",
        "bash -c 'rm x'",
        "$cmd = 'rm x'; & $cmd",
    ] {
        assert!(check_readonly(command).is_err(), "应拒绝: {command}");
    }
}

#[test]
fn absolute_paths_are_extracted_for_scope_check() {
    let paths = absolute_paths("rg -n Foo C:/repo/center/src --glob '*.java' -Path \"D:\\other\\dir\" /usr/lib/x");
    assert_eq!(paths, vec!["C:/repo/center/src", "D:\\other\\dir", "/usr/lib/x"]);
}

#[test]
fn git_read_forms_pass_and_write_forms_fail() {
    assert!(check_readonly("git branch -a").is_ok());
    assert!(check_readonly("git tag -l 'v*'").is_ok());
    assert!(check_readonly("git stash list").is_ok());
    assert!(check_readonly("git config --get user.name").is_ok());
    assert!(check_readonly("git branch feature").is_err(), "带位置参数是建分支");
    assert!(check_readonly("git tag v1").is_err());
    assert!(check_readonly("git stash pop").is_err());
    assert!(check_readonly("git remote add o url").is_err());
}

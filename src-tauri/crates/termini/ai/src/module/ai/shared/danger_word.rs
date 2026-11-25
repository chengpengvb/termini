const DANGER_STORAGE: &[&str] = &[
    "rm",
    "rm -r",
    "rm -f",
    "rm -rf",
    "rm -fr",
    "mv",
    "shred",
    "mkfs",
    "mkswap",
    "wipefs",
    "fdisk",
    "parted",
    "chattr",
    "dd if=",
    "dd of=",
    "of=/dev/",
    "> /dev/sd",
    "> /dev/vd",
    "> /dev/nvme",
    "-delete",
    "xargs rm",
    "truncate -s",
    "unlink",
];

const DANGER_PERMISSION: &[&str] = &["chmod -r", "chown -r", "chmod 777", "chmod 000", "takeown /f", "icacls /reset", "setfacl", "visudo", "sysctl -w", "setenforce", "usermod -g"];

const DANGER_WRITE_PATH: &[&str] =
    &["> /etc/", ">> /etc/", "tee /etc/", "> /boot/", ">> /boot/", "> /var/lib/", "> /usr/", "mv /etc", "rm /etc", "authorized_keys", "ssh-keygen -r", "> ~/.ssh"];

const DANGER_POWER: &[&str] = &["shutdown", "reboot", "halt", "poweroff", "init 0", "init 6", "stop-computer", "restart-computer", "wall"];

const DANGER_SERVICE: &[&str] = &[
    "systemctl stop",
    "systemctl restart",
    "systemctl disable",
    "systemctl mask",
    "systemctl reload",
    "stop-service",
    "set-service",
    "net stop",
    "kill -9",
    "kill",
    "killall",
    "pkill",
    "supervisorctl stop",
    "supervisorctl restart",
];

const DANGER_PACKAGE: &[&str] =
    &["apt remove", "apt purge", "apt upgrade", "apt-get remove", "apt-get purge", "yum remove", "yum update", "dnf remove", "rpm -e", "pip uninstall", "npm uninstall", "brew uninstall"];

const DANGER_ACCOUNT: &[&str] = &["useradd", "userdel", "usermod", "groupdel", "groupadd", "chpasswd", "passwd -d", "net user", "crontab -r", "crontab -e", "at rm"];

const DANGER_NETWORK: &[&str] = &[
    "iptables -f",
    "iptables -x",
    "iptables -d",
    "nft flush",
    "firewall-cmd --remove",
    "firewall-cmd --reload",
    "ip link set",
    "ip route del",
    "ip addr del",
    "route del",
    "ifdown",
    "ifconfig down",
    "nmcli connection delete",
];

const DANGER_DATABASE_SQL: &[&str] = &[
    "drop database",
    "drop table",
    "drop schema",
    "drop user",
    "drop index",
    "drop view",
    "drop tablespace",
    "drop partition",
    "drop column",
    "truncate table",
    "delete from",
    "insert into",
    "alter table",
    "alter system",
    "alter database",
    "create user",
    "grant all",
    "revoke",
    "set global",
    "flush privileges",
    "flush tables",
    "reset master",
    "reset slave",
    "stop slave",
    "start slave",
    "purge binary logs",
    "shutdown immediate",
    "vacuum full",
    "detach table",
    "dbcc",
    "sp_configure",
    "mysqladmin",
    "mysql_secure_installation",
    "dropdb",
    "dropuser",
    "pg_resetwal",
    "pg_ctl stop",
    "pg_ctl restart",
];

const DANGER_DATABASE_NOSQL: &[&str] = &[
    "flushall",
    "flushdb",
    "config set",
    "config rewrite",
    "debug segfault",
    "shutdown nosave",
    "script flush",
    "slaveof",
    "replicaof",
    "cluster reset",
    "cluster forget",
    "keys *",
    "dropdatabase",
    "deletemany",
    "deleteone",
    "db.shutdownserver",
    "_delete_by_query",
    "_forcemerge",
    "_close",
];

const DANGER_BACKUP: &[&str] = &[
    "rsync --delete",
    "lvremove",
    "vgremove",
    "pvremove",
    "lvreduce",
    "vgreduce",
    "mdadm --remove",
    "mdadm --zero-superblock",
    "zfs destroy",
    "zpool destroy",
    "zpool remove",
    "btrfs subvolume delete",
    "umount",
    "swapoff",
];

const DANGER_CONTAINER: &[&str] = &[
    "docker rm",
    "docker rmi",
    "docker stop",
    "docker kill",
    "docker restart",
    "docker update",
    "docker run",
    "docker prune",
    "docker system prune",
    "docker volume rm",
    "docker network rm",
    "docker compose down",
    "docker-compose down",
    "podman rm",
    "crictl rm",
    "crictl rmi",
    "nerdctl rm",
    "kubectl delete",
    "kubectl drain",
    "kubectl cordon",
    "kubectl taint",
    "kubectl apply",
    "kubectl create",
    "kubectl edit",
    "kubectl patch",
    "kubectl replace",
    "kubectl scale",
    "kubectl rollout",
    "kubectl set",
    "helm delete",
    "helm uninstall",
    "helm rollback",
    "helm upgrade",
    "helm install",
];

const DANGER_CLOUD: &[&str] = &["delete-", "terminate-", "deregister-", "detach-", "disassociate-", "revoke-", "reboot-", "aws s3 rb", "aws s3 rm", "s3 sync --delete", "put-object", "modify-"];

const DANGER_MIDDLEWARE: &[&str] = &[
    "nginx -s stop",
    "nginx -s reload",
    "nginx -s quit",
    "apachectl stop",
    "apachectl restart",
    "kafka-topics --delete",
    "kafka-configs --alter",
    "zkcli delete",
    "zkcli rmr",
    "deleteall",
    "rabbitmqctl stop",
    "rabbitmqctl purge_queue",
    "rabbitmqctl reset",
];

const DANGER_BIGDATA: &[&str] = &["hdfs dfs -rm", "hadoop fs -rm", "hdfs dfsadmin", "hdfs namenode -format", "hive -e drop", "msck repair", "spark-submit", "yarn application -kill"];

const DANGER_VIRTUAL: &[&str] = &["virsh destroy", "virsh undefine", "virsh shutdown", "esxcli", "vim-cmd", "vmware-cmd", "xe vm-shutdown", "qm destroy"];

const DANGER_CODE: &[&str] = &["git push", "git reset --hard", "git clean", "git checkout --", "git branch -d", "git rebase", "git filter-branch"];

const DANGER_WINDOWS: &[&str] = &[
    "del /f",
    "del /s",
    "del /q",
    "erase ",
    "rd /s",
    "rmdir /s",
    "diskpart",
    "bcdedit",
    "reg add",
    "reg delete",
    "cipher /w",
    "vssadmin",
    "wevtutil cl",
    "remove-item",
    "clear-content",
    "format-volume",
    "set-itemproperty",
    "new-itemproperty",
    "sc delete",
    "sc config",
];

const DANGER_DEVICE_CLI: &[&str] = &[
    "undo",
    "write erase",
    "write memory",
    "erase flash",
    "erase startup-config",
    "delete flash",
    "format flash",
    "format c:",
    "reset saved-configuration",
    "factory-reset",
    "clear config",
    "clear configuration",
    "configure terminal",
    "config t",
    "system-view",
    "copy tftp",
    "copy ftp",
    "save force",
    "reload",
];

const DANGER_HTTP_WRITE: &[&str] = &["-x delete", "-x put", "-x post", "-x patch", "--request delete", "--request put", "--request post", "-d @", "--data-binary"];

const DANGER_INJECT: &[&str] = &["| sh", "|sh", "| bash", "|bash", "base64 -d", "eval", "exec", "history -c", "chroot", ":(){"];

const DANGER_GROUPS: &[&[&str]] = &[
    DANGER_STORAGE,
    DANGER_PERMISSION,
    DANGER_WRITE_PATH,
    DANGER_POWER,
    DANGER_SERVICE,
    DANGER_PACKAGE,
    DANGER_ACCOUNT,
    DANGER_NETWORK,
    DANGER_DATABASE_SQL,
    DANGER_DATABASE_NOSQL,
    DANGER_BACKUP,
    DANGER_CONTAINER,
    DANGER_CLOUD,
    DANGER_MIDDLEWARE,
    DANGER_BIGDATA,
    DANGER_VIRTUAL,
    DANGER_CODE,
    DANGER_WINDOWS,
    DANGER_DEVICE_CLI,
    DANGER_HTTP_WRITE,
    DANGER_INJECT,
];

pub fn split_words(text: &str) -> Vec<String> {
    text.split(['\n', '\r', ',', '，', ';', '；']).map(|word| word.trim().to_lowercase()).filter(|word| !word.is_empty()).collect()
}

pub fn match_danger(command: &str, extra: &[String]) -> Option<String> {
    let target = normalize(command);
    if target.is_empty() {
        return None;
    }
    for group in DANGER_GROUPS {
        for word in *group {
            if contains_word(&target, word) {
                return Some(word.to_string());
            }
        }
    }
    for word in extra {
        let word = normalize(word);
        if !word.is_empty() && contains_word(&target, &word) {
            return Some(word);
        }
    }
    None
}

fn normalize(text: &str) -> String {
    text.to_lowercase().split_whitespace().collect::<Vec<&str>>().join(" ")
}

fn contains_word(target: &str, word: &str) -> bool {
    target.match_indices(word).any(|(start, _)| head_free(target, start, word) && tail_free(target, start + word.len(), word))
}

fn head_free(target: &str, start: usize, word: &str) -> bool {
    if !word.starts_with(is_word_char) {
        return true;
    }
    !target[..start].chars().next_back().is_some_and(is_word_char)
}

fn tail_free(target: &str, end: usize, word: &str) -> bool {
    if !word.ends_with(is_word_char) {
        return true;
    }
    !target[end..].chars().next().is_some_and(is_word_char)
}

fn is_word_char(value: char) -> bool {
    value.is_alphanumeric() || value == '_' || value == '-'
}

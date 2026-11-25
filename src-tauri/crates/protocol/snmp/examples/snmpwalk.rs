use std::str::FromStr;
use std::time::Duration;
use protocol_snmp::{Oid, SnmpClient, SnmpClientConfig, SnmpVersion};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        eprintln!("usage: snmpwalk <host> <port> <community> <get|walk> <oid...> [--v1]");
        std::process::exit(2);
    }
    let host = args[1].clone();
    let port: u16 = args[2].parse()?;
    let community = args[3].clone();
    let op = args[4].clone();
    let oids: Vec<Oid> = args[5..].iter().filter(|raw| !raw.starts_with("--")).map(|raw| Oid::from_str(raw)).collect::<Result<_, _>>()?;
    let version = if args.iter().any(|arg| arg == "--v1") { SnmpVersion::V1 } else { SnmpVersion::V2c };

    let client = SnmpClient::connect(SnmpClientConfig { host, port, version, community, timeout: Duration::from_secs(3), retries: 1 }).await?;

    let rows = match op.as_str() {
        "get" => client.get(&oids).await?,
        "walk" => client.walk(&oids[0], 200).await?,
        other => anyhow::bail!("unknown op: {}", other),
    };
    for row in &rows {
        println!("{} = {}: {}", row.oid, row.value.type_name(), row.value);
    }
    eprintln!("rows: {}", rows.len());
    Ok(())
}

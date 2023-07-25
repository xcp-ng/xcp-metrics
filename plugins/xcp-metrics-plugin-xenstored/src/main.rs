use xenstore_rs::{XBTransaction, Xs, XsOpenFlags};

fn main() -> anyhow::Result<()> {
    let xs = Xs::new(XsOpenFlags::ReadOnly).map_err(|e| anyhow::anyhow!("{e}"))?;

    for dir in xs.directory(XBTransaction::Null, "/")? {
        println!("{dir}");
    }

    Ok(())
}

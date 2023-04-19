use std::path::PathBuf;

const AB: &str = env!("CARGO_PKG_DESCRIPTION");
const AU: &str = env!("CARGO_PKG_AUTHORS");

#[derive(clap::Parser, Debug)]
#[clap(about = AB, author = AU)]
pub struct Args {
  
}

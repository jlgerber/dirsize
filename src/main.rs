use dirsize::{get_dirsize, Opt};
use structopt::StructOpt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Opt::from_args();
    let mut dirsize = get_dirsize(args)?;
    println!("Total size of directory:\n    {}", dirsize.size);
    println!("Total number of files:\n      {}", dirsize.file_cnt);
    if dirsize.has_errors() {
        println!("Problem reading metadata from file:");
        for error in dirsize.take_errors().unwrap() {
            println!("\t{}", error.display());
        }
    }
    Ok(())
}

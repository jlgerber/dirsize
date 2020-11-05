use byte_unit::ByteUnit;
use dirsize::{get_dirsize, DirsizeRequest};
use std::time::Instant;
use structopt::StructOpt;

/// dirsize calculates the cumulative size taken up by
/// a supplied directory's contents.
/// dirsize does not follow or read symlinks when calculating this size.
/// dirsize does attempt to adjust sizing by dividing the size of each file by
/// its hard link count. If all of the hardlinks are not contained within the
/// provided directory, this may result in underestimation.
#[derive(StructOpt, Debug)]
pub struct Opt {
    /// Number of threads to use, defaulting to available threads
    #[structopt(short = "t", long = "threads")]
    threads: Option<usize>,
    /// Path to operate upon
    path: String,
    /// Print the size and name of each filepath as we scan it
    #[structopt(short = "v", long = "verbose")]
    verbose: bool,
    /// Unit to output data in, defaulting to MB. (b, k, kib, mb, mib, gb, gib, tb, tib, pb, pib)
    #[structopt(short = "u", long = "unit")]
    unit: Option<ByteUnit>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Opt {
        threads,
        path,
        verbose,
        unit,
    } = Opt::from_args();

    let before = Instant::now();

    let mut dirsize = get_dirsize(DirsizeRequest {
        path,
        threads,
        verbose,
        unit,
    })?;
    
    println!("\nTotal size of directory:\n    {}", dirsize.size);
    println!("\nTotal number of files:  \n    {}", dirsize.file_cnt);
    println!("\n Elapsed Time:          \n    {:.2?}", before.elapsed());

    if dirsize.has_errors() {
        println!("Problem reading metadata from file:");
        for error in dirsize.take_errors().unwrap() {
            println!("\t{}", error);
        }
    }
    Ok(())
}

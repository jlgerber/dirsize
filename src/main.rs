use byte_unit::ByteUnit;
use dirsize::get_dirsize;
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
    /// Print the name of each filepath as we scan it
    #[structopt(short = "d", long = "debug")]
    debug: bool,
    /// Unit to output data in, defaulting to MB. (b, k, kib, mb, mib, gb, gib, tb, tib, pb, pib)
    #[structopt(short = "u", long = "unit")]
    unit: Option<ByteUnit>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Opt {
        threads,
        path,
        debug,
        unit,
    } = Opt::from_args();

    let mut dirsize = get_dirsize(path, threads, debug, unit)?;
    println!("Total size of directory:\n    {}", dirsize.size);
    println!("Total number of files:\n      {}", dirsize.file_cnt);
    if dirsize.has_errors() {
        println!("Problem reading metadata from file:");
        for error in dirsize.take_errors().unwrap() {
            println!("\t{}", error);
        }
    }
    Ok(())
}

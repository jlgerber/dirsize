use byte_unit::{AdjustedByte, Byte, ByteUnit};
use ignore::WalkBuilder;
use ignore::WalkState::*;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
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

pub struct DirSize {
    pub size: AdjustedByte,
    pub file_cnt: usize,
    errors: Option<Vec<PathBuf>>,
}

impl DirSize {
    pub fn has_errors(&self) -> bool {
        self.errors.is_some()
    }
    /// retrieve the errors
    pub fn take_errors(&mut self) -> Option<Vec<PathBuf>> {
        self.errors.take()
    }
}

pub fn get_dirsize(args: Opt) -> Result<DirSize, Box<dyn std::error::Error>> {
    let debug = args.debug;
    let unit = args.unit.unwrap_or(ByteUnit::GB);
    let threads = args.threads.unwrap_or(0);

    let total_size = Arc::new(AtomicUsize::new(0));
    let file_cnt = Arc::new(AtomicUsize::new(0));
    let errors: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(Vec::new()));
    let _ = WalkBuilder::new(args.path)
        .ignore(false)
        .threads(threads)
        .git_global(false)
        .git_ignore(false)
        .git_exclude(false)
        .follow_links(false)
        .parents(false)
        .filter_entry(|entry| !entry.path_is_symlink())
        .build_parallel()
        .run(|| {
            let total_size_c = Arc::clone(&total_size);
            let file_cnt_c = Arc::clone(&file_cnt);
            let errors_c = Arc::clone(&errors);
            Box::new(move |result| {
                let pp = result.unwrap();
                let p = pp.path();
                let metadata = fs::metadata(p);
                if debug {
                    eprintln!("path {:?}", &p);
                }
                match metadata {
                    Ok(meta) => {
                        let adjusted_size = meta.len() / meta.nlink();
                        total_size_c.fetch_add(adjusted_size as usize, Ordering::SeqCst);
                        file_cnt_c.fetch_add(1, Ordering::SeqCst);
                    }
                    _ => {
                        let mut v = errors_c.lock().unwrap();
                        v.push(p.to_path_buf());
                    }
                };
                Continue
            })
        });
    let total_size = Byte::from_bytes(
        Arc::try_unwrap(total_size)
            .map_err(|_| "problem unwrapping file size")?
            .into_inner() as u128,
    );
    let total_size = total_size.get_adjusted_unit(unit);
    let file_cnt = Arc::try_unwrap(file_cnt)
        .map_err(|_| "problem unwrapping file count")?
        .into_inner();
    let errors = Arc::try_unwrap(errors)
        .map_err(|_| "problem unwrapping errors")?
        .into_inner()?;
    let errors = if errors.len() > 0 { Some(errors) } else { None };

    Ok(DirSize {
        size: total_size,
        file_cnt,
        errors,
    })
}

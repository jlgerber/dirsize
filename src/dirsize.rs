use crate::DirsizeError;
use byte_unit::{AdjustedByte, Byte, ByteUnit};
use ignore::Error;
use ignore::WalkBuilder;
use ignore::WalkState::*;
use pad::PadStr;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// struct which packages the returned information from
/// the request to get the total size of a directory.
pub struct DirSize {
    /// The total size, expressed in the unit encoded in
    /// the type
    pub size: AdjustedByte,
    /// Total number of files counted
    pub file_cnt: usize,
    /// An optional list of errors encountered
    errors: Option<Vec<DirsizeError>>,
}

impl DirSize {
    /// Did we encounter any errors?
    pub fn has_errors(&self) -> bool {
        self.errors.is_some()
    }
    /// retrieve the errors
    pub fn take_errors(&mut self) -> Option<Vec<DirsizeError>> {
        self.errors.take()
    }
}

/// Request to calculate dirsize, consiting of a target
/// path and an assortment of options which modify the
/// behavior
pub struct DirsizeRequest {
    pub path: String,
    pub threads: Option<usize>,
    pub verbose: bool,
    pub unit: Option<ByteUnit>,
}

/// Given a DirsizeRequest, consisting of a target
/// directory and an assortment of options, calculate
/// the total size of a directory's contents and return
/// it allong with the total number of files visited
/// and any files that we were not able to read metadata
/// for.
pub fn get_dirsize(request: DirsizeRequest) -> Result<DirSize, Box<dyn std::error::Error>> {
    let DirsizeRequest {
        path,
        threads,
        verbose,
        unit,
    } = request;

    let unit = unit.unwrap_or(ByteUnit::GB);
    let threads = threads.unwrap_or(0);

    // Create variables to send to various threads.
    // Each must be entombed in an Arc<Mutex>
    // to survive the journey
    let total_size = Arc::new(AtomicUsize::new(0));
    let file_cnt = Arc::new(AtomicUsize::new(0));
    let errors: Arc<Mutex<Vec<DirsizeError>>> = Arc::new(Mutex::new(Vec::new()));
    let _ = WalkBuilder::new(path)
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
            // create clones to satisfy the Arc requirements
            let total_size_c = Arc::clone(&total_size);
            let file_cnt_c = Arc::clone(&file_cnt);
            let errors_c = Arc::clone(&errors);
            // box up a result to be sent to each thread
            Box::new(move |result| {
                if result.is_ok() {
                    let pp = result.unwrap();
                    let p = pp.path();
                    let metadata = fs::metadata(p);
                    match metadata {
                        Ok(meta) => {
                            let adjusted_size = meta.len() / meta.nlink();
                            if verbose {
                                let sz = Byte::from_bytes(adjusted_size as u128)
                                    .get_appropriate_unit(false)
                                    .to_string()
                                    .pad_to_width(16);
                                println!("{} {}", &sz, &p.to_str().expect("couldnt unwrap"));
                            }
                            total_size_c.fetch_add(adjusted_size as usize, Ordering::SeqCst);
                            file_cnt_c.fetch_add(1, Ordering::SeqCst);
                        }
                        _ => {
                            let mut v = errors_c.lock().expect("Unable to lock mutex.");
                            v.push(DirsizeError::Metadata(p.to_path_buf()));
                        }
                    };
                } else {
                    match result {
                        Err(Error::WithDepth { err, .. }) => match *err {
                            Error::WithPath { path, .. } => {
                                let mut v = errors_c.lock().expect("Unable to lock mutex");
                                v.push(DirsizeError::PermissionDenied(path));
                            }
                            _ => {
                                let mut v = errors_c.lock().expect("Unable to lock mutex");
                                v.push(DirsizeError::UnknownError);
                            }
                        },
                        _ => {
                            let mut v = errors_c.lock().expect("Unable to lock mutex");
                            v.push(DirsizeError::UnknownError);
                        }
                    };
                }
                Continue
            })
        });

    // calculate the results
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

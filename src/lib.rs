use byte_unit::{AdjustedByte, Byte, ByteUnit};
use ignore::WalkBuilder;
use ignore::WalkState::*;
use ignore::Error;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub mod error;
pub use error::DirsizeError;

/// Custom return type
pub struct DirSize {
    pub size: AdjustedByte,
    pub file_cnt: usize,
    errors: Option<Vec<DirsizeError>>,
}

impl DirSize {
    pub fn has_errors(&self) -> bool {
        self.errors.is_some()
    }
    /// retrieve the errors
    pub fn take_errors(&mut self) -> Option<Vec<DirsizeError>> {
        self.errors.take()
    }
}

/// Given a path, and assorted arguments, calculate 
/// the total size of a directory's contents and return
/// it allong with the total number of files visited 
/// and any files that we were not able to read metadata
/// for. 
pub fn get_dirsize(
    path: String,
    threads: Option<usize>,
    debug: bool,
    unit: Option<ByteUnit>,
) -> Result<DirSize, Box<dyn std::error::Error>> {
    let unit = unit.unwrap_or(ByteUnit::GB);
    let threads = threads.unwrap_or(0);

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
            let total_size_c = Arc::clone(&total_size);
            let file_cnt_c = Arc::clone(&file_cnt);
            let errors_c = Arc::clone(&errors);
            Box::new(move |result| {
                if result.is_ok() {
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
                            let mut v = errors_c.lock().expect("Unable to lock mutex.");
                            v.push(DirsizeError::Metadata(p.to_path_buf()));
                        }
                    };
                } else {
                    match result {
                        Err(Error::WithDepth{err,..}) => {
                            match *err {
                                Error::WithPath{path,..} => {
                                    let mut v = errors_c.lock().expect("Unable to lock mutex");
                                    v.push(DirsizeError::PermissionDenied(path)); 
                                },
                                _ => {
                                    let mut v = errors_c.lock().expect("Unable to lock mutex");
                                    v.push(DirsizeError::UnknownError); 
                                }
                            }
                      
                        }
                        _ => {
    
                            let mut v = errors_c.lock().expect("Unable to lock mutex");
                            v.push(DirsizeError::UnknownError); 
                        }
                    };
                }
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

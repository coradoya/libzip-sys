#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub type ZipResult<T> = Result<T, Box<dyn Error + Sync + Send>>;

use std::error::Error;
use std::ffi::{CStr, CString};
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;

#[cfg(test)]
use mockall::automock;

#[derive(Debug, Default)]
pub struct Zip {
    file: Option<*mut zip_t>,
    filename: PathBuf
}

#[derive(Clone, Debug)]
pub struct ZipEntry {
    name: String,
    index: u64,
}

#[cfg_attr(test, automock)]
pub trait ZipFile {
    fn add_buffer(&self, data: &String, filename: &str) -> ZipResult<()>;
    fn add_file(&self, src: &Path, filename: &str) -> ZipResult<()>;
    fn close(&self) -> ZipResult<()>;
    fn entries(&self) -> ZipResult<Vec<ZipEntry>>;
    fn open(file: &PathBuf) -> ZipResult<Zip>;
}

#[cfg_attr(test, automock)]
pub trait ZipPack {
    fn pack_file(batch_name: String, src: &str, filename: String);
}

impl Zip {
    pub fn filename(&self) -> &Path {
        self.filename.as_path()
    }
}

impl ZipFile for Zip {
    fn add_buffer(&self, data: &String, filename: &str) -> ZipResult<()> {
        let c_filename = CString::new(filename).unwrap();
        match self.file {
            Some(zip_file) => {
                unsafe {
                    let zip_source_err = null_mut();
                    let zip_source = zip_source_buffer_create(
                        data.as_ptr() as _,
                        data.len() as zip_uint64_t,
                        0,
                        zip_source_err,
                    );
                    let zip_result = zip_file_add(
                        zip_file,
                        c_filename.as_ptr(),
                        zip_source,
                        ZIP_FL_OVERWRITE | ZIP_FL_ENC_UTF_8,
                    );
                    match zip_result {
                        -1 => Err("Unable to add buffer to the zip".into()),
                        _ => Ok(()),
                    }
                }
            }
            None => Err("Zip file is not open".into())
        }
    }

    fn add_file(&self, src: &Path, filename: &str) -> ZipResult<()> {
        let c_src = CString::new(src.to_str().unwrap()).unwrap();
        let c_filename = CString::new(filename).unwrap();

        match self.file {
            Some(zip_file) => {
                unsafe {
                    let zip_source_err = null_mut();
                    let zip_source = zip_source_file_create(c_src.as_ptr(), 0, -1, zip_source_err);
                    let zip_result = zip_file_add(
                        zip_file,
                        c_filename.as_ptr(),
                        zip_source,
                        ZIP_FL_OVERWRITE | ZIP_FL_ENC_UTF_8,
                    );

                    if zip_result == -1 {
                        return Err("Unable to add file to zip".into());
                    }
                }
                Ok(())
            }
            None => Err("Zip file is not open".into())
        }
    }

    fn close(&self) -> ZipResult<()> {
        match self.file {
            Some(zip_file) => {
                unsafe {
                    let result = zip_close(zip_file);

                    match result {
                        0 => Ok(()),
                        _ => Err("Unable to close the zip file".into()),
                    }
                }
            }
            None => Err("No file to close".into())
        }
    }

    fn entries(&self) -> ZipResult<Vec<ZipEntry>> {
        if let Some(zip_file) = self.file {
            let num_entries = unsafe {
                zip_get_num_entries(zip_file, 0)
            };

            let mut entries = Vec::new();
            if let Ok(num_entries) = zip_uint64_t::try_from(num_entries) {
                for index in 0..num_entries {
                    let name = unsafe {
                        let name = zip_get_name(zip_file.clone(), index.clone(), ZIP_FL_ENC_GUESS);
                        CStr::from_ptr(name).to_str()
                    };

                    if let Ok(name) = name {
                        let entry = ZipEntry {
                            name: String::from(name),
                            index
                        };
                        entries.push(entry);
                    }
                }

                Ok(entries)
            } else {
                Err("Invalid number of entries".into())
            }
        } else {
            Ok(Vec::new())
        }
    }

    fn open(file: &PathBuf) -> ZipResult<Zip> {
        let zip_file;
        let location: &str = file.to_str().unwrap();
        let c_src = CString::new(location)?;
        unsafe {
            let zip_file_err = null_mut();
            zip_file = zip_open(c_src.as_ptr(), ZIP_CHECKCONS as c_int, zip_file_err);
        }

        Ok(Zip {
            file: Some(zip_file),
            filename: file.clone()
        })
    }
}

impl ZipPack for Zip {
    fn pack_file(batch_name: String, src: &str, filename: String) {
        let c_batch_name = CString::new(batch_name).unwrap();
        let c_src = CString::new(src).unwrap();
        let c_filename = CString::new(filename).unwrap();

        unsafe {
            let zip_file_err = null_mut();
            let zip_file = zip_open(c_batch_name.as_ptr(), ZIP_CREATE as c_int, zip_file_err);

            let zip_source_err = null_mut();
            let zip_source = zip_source_file_create(c_src.as_ptr(), 0, -1, zip_source_err);

            let zip_result = zip_file_add(
                zip_file,
                c_filename.as_ptr(),
                zip_source,
                ZIP_FL_OVERWRITE | ZIP_FL_ENC_UTF_8,
            );

            if zip_result == -1 {
                panic!("Unable to add zip file {}", src);
            }
            zip_close(zip_file);
        }
    }
}

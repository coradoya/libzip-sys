#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::error::Error;
use std::ffi::CString;
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

#[cfg_attr(test, automock)]
pub trait ZipFile {
    fn add_buffer(&self, data: &String, filename: &str) -> Result<(), Box<dyn Error + Sync + Send>>;
    fn add_file(&self, src: &Path, filename: &str) -> Result<(), Box<dyn Error + Sync + Send>>;
    fn close(&self) -> Result<(), Box<dyn Error + Sync + Send>>;
    fn open(file: &PathBuf) -> Result<Zip, Box<dyn Error + Sync + Send>>;
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
    fn add_buffer(&self, data: &String, filename: &str) -> Result<(), Box<dyn Error + Sync + Send>> {
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

    fn add_file(&self, src: &Path, filename: &str) -> Result<(), Box<dyn Error + Sync + Send>> {
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

    fn close(&self) -> Result<(), Box<dyn Error + Sync + Send>> {
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

    fn open(file: &PathBuf) -> Result<Zip, Box<dyn Error + Sync + Send>> {
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

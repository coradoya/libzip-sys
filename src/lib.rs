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

pub struct ZipFile {
    file: *mut zip_t,
    filename: String
}

#[cfg_attr(test, automock)]
pub trait ZipFileTrait {
    fn add_buffer(&self, data: &String, filename: &str) -> Result<(), Box<dyn Error + Sync + Send>>;
    fn add_file(&self, src: &Path, filename: &str) -> Result<(), Box<dyn Error + Sync + Send>>;
    fn close(&self) -> Result<(), Box<dyn Error + Sync + Send>>;
    fn open(file: &Path) -> Result<ZipFile, Box<dyn Error + Sync + Send>>;
    fn pack_file(&self, batch_name: String, src: &str, filename: String);
}

impl ZipFileTrait for ZipFile {
    fn add_buffer(&self, data: &String, filename: &str) -> Result<(), Box<dyn Error + Sync + Send>> {
        let c_filename = CString::new(filename).unwrap();
        unsafe {
            let zip_source_err = null_mut();
            let zip_source = zip_source_buffer_create(
                data.as_ptr() as _,
                data.len() as zip_uint64_t,
                0,
                zip_source_err,
            );
            let zip_result = zip_file_add(
                self.file,
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

    fn add_file(&self, src: &Path, filename: &str) -> Result<(), Box<dyn Error + Sync + Send>> {
        let c_src = CString::new(src.to_str().unwrap()).unwrap();
        let c_filename = CString::new(filename).unwrap();

        unsafe {
            let zip_source_err = null_mut();
            let zip_source = zip_source_file_create(c_src.as_ptr(), 0, -1, zip_source_err);
            let zip_result = zip_file_add(
                self.file,
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

    fn close(&self) -> Result<(), Box<dyn Error + Sync + Send>> {
        unsafe {
            let result = zip_close(self.file);

            match result {
                0 => Ok(()),
                _ => Err("Unable to close the zip file".into()),
            }
        }
    }

    fn open(file: &Path) -> Result<ZipFile, Box<dyn Error + Sync + Send>> {
        let zip_file;
        let location: &str = file.to_str().unwrap();
        let c_src = CString::new(location)?;
        unsafe {
            let zip_file_err = null_mut();
            zip_file = zip_open(c_src.as_ptr(), ZIP_CHECKCONS as c_int, zip_file_err);
        }

        Ok(ZipFile {
            file: zip_file,
            filename: file.to_str().unwrap().to_string()
        })
    }

    fn pack_file(&self, batch_name: String, src: &str, filename: String) {
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

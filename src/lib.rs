#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use lambda_runtime::Error;
use std::ffi::CString;
use std::fmt::Error;
use std::os::raw::c_int;
use std::path::Path;
use std::ptr::null_mut;

#[cfg(test)]
use mockall::automock;

pub struct ZipFile {
    file: String,
    zip_file: *mut zip_t,
}

#[cfg_attr(test, automock)]
pub trait ZipFileTrait {
    fn add_buffer(zip_file: *mut zip_t, data: &String, filename: &str) -> Result<(), Error>;
    fn add_file(zip_file: *mut zip_t, src: &Path, filename: &str) -> Result<(), Error>;
    fn close(zip_file: *mut zip_t) -> Result<(), Error>;
    fn open(file: &Path) -> Result<*mut zip_t, Error>;
}

impl ZipFileTrait for ZipFile {
    fn add_buffer(zip_file: *mut zip_t, data: &String, filename: &str) -> Result<(), Error> {
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

    fn add_file(zip_file: *mut zip_t, src: &Path, filename: &str) -> Result<(), Error> {
        let c_src = CString::new(src.to_str().unwrap()).unwrap();
        let c_filename = CString::new(filename).unwrap();

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

    fn close(zip_file: *mut zip_t) -> Result<(), Error> {
        unsafe {
            let result = zip_close(zip_file);

            match result {
                0 => Ok(()),
                _ => Err("Unable to close the zip file".into()),
            }
        }
    }

    fn open(file: &Path) -> Result<*mut zip_t, Error> {
        let zip_file;
        let location: &str = file.to_str().unwrap();
        let c_src = CString::new(location)?;
        unsafe {
            let zip_file_err = null_mut();
            zip_file = zip_open(c_src.as_ptr(), ZIP_CHECKCONS as c_int, zip_file_err);
        }

        Ok(zip_file)
    }
}

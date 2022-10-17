#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
// include!("zip.rs");

pub type ZipResult<T> = Result<T, Box<dyn Error + Sync + Send>>;

use std::error::Error;
use std::ffi::{c_void, CStr, CString};
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use tracing::{event, info, Level};

#[cfg(test)]
use mockall::automock;

#[derive(Clone, Debug, Default)]
pub struct Zip {
    file: Option<*mut zip_t>,
    filename: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ZipEntry<'a> {
    zip_file: &'a Zip,
    file: Option<*mut zip_file_t>,
    name: String,
}

#[cfg_attr(test, automock)]
pub trait ZipFile {
    fn add_buffer(&self, data: &String, filename: &str) -> ZipResult<()>;
    fn add_file(&self, src: &Path, filename: &str) -> ZipResult<()>;
    fn close(&self) -> ZipResult<()>;
    fn entries(&self) -> ZipResult<Vec<ZipEntry>>;
    fn open(file: &PathBuf, create: bool) -> ZipResult<Zip>;
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
            None => Err("Zip file is not open".into()),
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
            None => Err("Zip file is not open".into()),
        }
    }

    fn close(&self) -> ZipResult<()> {
        match self.file {
            Some(zip_file) => unsafe {
                let result = zip_close(zip_file);

                match result {
                    0 => Ok(()),
                    _ => {
                        let msg = zip_strerror(zip_file);
                        let msg = CStr::from_ptr(msg).to_str()?;
                        Err(msg.into())
                    },
                }
            },
            None => Err("No file to close".into()),
        }
    }

    fn entries(&self) -> ZipResult<Vec<ZipEntry>> {
        if let Some(zip_file) = self.file {
            let num_entries = unsafe { zip_get_num_entries(zip_file, 0) };

            let mut entries = Vec::new();
            if let Ok(num_entries) = zip_uint64_t::try_from(num_entries) {
                for index in 0..num_entries {
                    let name = unsafe {
                        let name = zip_get_name(zip_file, index.clone(), ZIP_FL_ENC_GUESS);
                        CStr::from_ptr(name).to_str()
                    };

                    if let Ok(name) = name {
                        let entry = ZipEntry {
                            name: String::from(name),
                            zip_file: self,
                            file: None
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

    fn open(file: &PathBuf, create: bool) -> ZipResult<Zip> {
        let zip_file;
        let location: &str = file.to_str().unwrap();
        let c_src = CString::new(location)?;
        unsafe {
            let zip_file_err: *mut c_int = null_mut();
            let flags = if create {
                ZIP_CHECKCONS as c_int | ZIP_CREATE as c_int
            } else {
                ZIP_CHECKCONS as c_int
            };
            zip_file = zip_open(c_src.as_ptr(), flags, zip_file_err);

            if zip_file.is_null() {
                match zip_file_err.read() as u32 {
                    ZIP_ER_EXISTS => { Err("The file specified by path exists and ZIP_EXCL is set.".into()) }
                    ZIP_ER_INCONS => { Err("Inconsistencies were found in the file specified by path..".into()) }
                    ZIP_ER_INVAL => { Err("The path argument is NULL".into()) }
                    ZIP_ER_MEMORY => { Err("Required memory could not be allocated".into()) }
                    ZIP_ER_NOENT => { Err("The file specified by path does not exist and ZIP_CREATE is not set".into()) }
                    ZIP_ER_NOZIP => { Err("The file specified by path is not a zip archive".into()) }
                    ZIP_ER_OPEN => { Err("The file specified by path could not be opened".into()) }
                    ZIP_ER_READ => { Err("A read error ocurred".into()) }
                    ZIP_ER_SEEK => { Err("The file specified by path does not allow seeks".into()) }
                    _ => { Err("Unexpected error while trying to open the zip".into()) }
                 }
            } else {
                Ok(Zip {
                    file: Some(zip_file),
                    filename: file.clone(),
                })
            }
        }
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

impl ZipEntry<'_> {
    pub fn close(&mut self) {
        if let Some(file) = self.file {
            unsafe {
                zip_fclose(file);
            }
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn open(&mut self) -> ZipResult<()> {
        match self.zip_file.file {
            Some(zip_file) => {
                let fname = CString::new(self.name())?;

                let file = unsafe { zip_fopen(zip_file, fname.as_ptr(), ZIP_FL_ENC_GUESS) };

                if file.is_null() {
                    Err("Unable to open file in zip".into())
                } else {
                    self.file = Some(file);

                    Ok(())
                }
            }
            None => Err("Zip file is not valid. Was it opened?".into())
        }
    }
}

impl std::io::Read for ZipEntry<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.file {
            Some(zip_file) => {
                let block_size = buf.len() as u64;
                let bytes_readed = unsafe {
                    zip_fread(zip_file, buf.as_mut_ptr() as *mut c_void, block_size)
                };

                if bytes_readed >= 0 {
                    Ok(bytes_readed as usize)
                } else {
                    Err(std::io::Error::new(std::io::ErrorKind::Other, "Unable to read data"))
                }
            }
            None => Err(std::io::Error::new(std::io::ErrorKind::Other, "Zip file is not open"))
        }
    }
}

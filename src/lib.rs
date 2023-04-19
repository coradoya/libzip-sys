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
use std::pin::Pin;
use std::ptr::null_mut;
use std::task::{Context, Poll};
use tokio::io::ReadBuf;

#[derive(Clone, Debug, Default)]
pub struct Zip {
    file: Option<*mut zip_t>,
    filename: PathBuf,
}

#[derive(Debug)]
pub struct Entry<'a> {
    zip_file: &'a Zip,
    file: Option<*mut zip_file_t>,
    name: String,
    is_open: bool,
}

pub trait ZipFile {
    fn add_buffer(&self, data: &str, filename: &str) -> ZipResult<()>;
    fn add_file(&self, src: &Path, filename: &str) -> ZipResult<()>;
    fn close(&self) -> ZipResult<()>;
    fn delete_file(&self, filename: &str) -> ZipResult<()>;
    fn entries(&self) -> ZipResult<Vec<Box<dyn ZipEntry + '_>>>;
    fn filename(&self) -> &Path;
    fn get_error(&self, code: i64) -> ZipResult<()>;
    fn open(&mut self, file: &Path, create: bool) -> ZipResult<()>;
    fn get_entry(&self, entry_name: &str) -> Option<Box<dyn ZipEntry + '_>>;
}

pub trait ZipEntry: std::io::Read {
    fn close(&mut self);
    fn name(&self) -> String;
    fn open(&mut self) -> ZipResult<()>;
}

pub trait ZipPack {
    fn pack_file(batch_name: String, src: &str, filename: String);
}

impl ZipFile for Zip {
    fn add_buffer(&self, data: &str, filename: &str) -> ZipResult<()> {
        let c_filename = CString::new(filename).unwrap();
        match self.file {
            Some(zip_file) => unsafe {
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
            },
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
                    }
                }
            },
            None => Err("No file to close".into()),
        }
    }

    fn delete_file(&self, filename: &str) -> ZipResult<()> {
        let file = match self.file {
            None => return Err("No zip is open".into()),
            Some(file) => file,
        };

        let filename = CString::new(filename)?;

        let file_stat = zip_stat_t {
            valid: 0,
            name: CString::new("")?.as_ptr(),
            index: 0,
            size: 0,
            comp_size: 0,
            mtime: 0,
            crc: 0,
            comp_method: 0,
            encryption_method: 0,
            flags: 0,
        };
        let file_stat = Box::new(file_stat);
        let file_stat = Box::into_raw(file_stat);

        let file_stat = unsafe {
            zip_stat_init(file_stat);
            let result = zip_stat(file, filename.as_ptr(), ZIP_FL_ENC_GUESS, file_stat);
            self.get_error(result as i64)?;
            Box::from_raw(file_stat)
        };

        let result = unsafe { zip_delete(file, file_stat.index) };
        self.get_error(result as i64)?;

        Ok(())
    }

    fn entries(&self) -> ZipResult<Vec<Box<dyn ZipEntry + '_>>> {
        if let Some(zip_file) = self.file {
            let num_entries = unsafe { zip_get_num_entries(zip_file, 0) };

            let mut entries = Vec::new();
            if let Ok(num_entries) = zip_uint64_t::try_from(num_entries) {
                for index in 0..num_entries {
                    let name = unsafe {
                        let name = zip_get_name(zip_file, index, ZIP_FL_ENC_GUESS);
                        CStr::from_ptr(name).to_str()
                    };

                    if let Ok(name) = name {
                        let entry: Box<dyn ZipEntry> = Box::new(Entry {
                            name: String::from(name),
                            zip_file: self,
                            file: None,
                            is_open: false,
                        });
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

    fn filename(&self) -> &Path {
        self.filename.as_path()
    }

    fn get_error(&self, code: i64) -> ZipResult<()> {
        if code == 0 {
            return Ok(());
        }

        let file = match self.file {
            None => return Err("Zip file not open".into()),
            Some(file) => file,
        };

        let error = unsafe {
            let error = zip_strerror(file);
            CStr::from_ptr(error).to_str()?
        };

        Err(String::from(error).into())
    }

    fn open(&mut self, file: &Path, create: bool) -> ZipResult<()> {
        let zip_file;
        let location: &str = file.to_str().unwrap();
        let c_src = CString::new(location)?;
        unsafe {
            let mut zip_file_err = 0i32;
            let flags = if create {
                ZIP_CHECKCONS as c_int | ZIP_CREATE as c_int
            } else {
                ZIP_CHECKCONS as c_int
            };
            zip_file = zip_open(c_src.as_ptr(), flags, &mut zip_file_err as *mut c_int);

            if zip_file.is_null() {
                match zip_file_err as u32 {
                    ZIP_ER_EXISTS => {
                        Err("The file specified by path exists and ZIP_EXCL is set.".into())
                    }
                    ZIP_ER_INCONS => {
                        Err("Inconsistencies were found in the file specified by path..".into())
                    }
                    ZIP_ER_INVAL => Err("The path argument is NULL".into()),
                    ZIP_ER_MEMORY => Err("Required memory could not be allocated".into()),
                    ZIP_ER_NOENT => Err(
                        "The file specified by path does not exist and ZIP_CREATE is not set"
                            .into(),
                    ),
                    ZIP_ER_NOZIP => Err("The file specified by path is not a zip archive".into()),
                    ZIP_ER_OPEN => Err("The file specified by path could not be opened".into()),
                    ZIP_ER_READ => Err("A read error ocurred".into()),
                    ZIP_ER_SEEK => Err("The file specified by path does not allow seeks".into()),
                    _ => Err("Unexpected error while trying to open the zip".into()),
                }
            } else {
                self.file = Some(zip_file);
                self.filename = file.into();

                Ok(())
            }
        }
    }

    fn get_entry(&self, entry_name: &str) -> Option<Box<dyn ZipEntry + '_>> {
        self.entries()
            .unwrap_or_else(|_| vec![])
            .into_iter()
            .find(|entry| entry.name().eq(entry_name))
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
                panic!("Unable to add zip file {src}");
            }
            zip_close(zip_file);
        }
    }
}

impl ZipEntry for Entry<'_> {
    fn close(&mut self) {
        if !self.is_open {
            return;
        }

        if let Some(file) = self.file {
            unsafe {
                zip_fclose(file);
            }
            self.is_open = false;
        }
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn open(&mut self) -> ZipResult<()> {
        match self.zip_file.file {
            Some(zip_file) => {
                let fname = CString::new(self.name())?;

                let file = unsafe { zip_fopen(zip_file, fname.as_ptr(), ZIP_FL_ENC_GUESS) };

                if file.is_null() {
                    Err("Unable to open file in zip".into())
                } else {
                    self.file = Some(file);
                    self.is_open = true;

                    Ok(())
                }
            }
            None => Err("Zip file is not valid. Was it opened?".into()),
        }
    }
}

impl std::io::Read for Entry<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.file {
            Some(zip_file) => {
                let block_size = buf.len() as u64;
                let bytes_readed =
                    unsafe { zip_fread(zip_file, buf.as_mut_ptr() as *mut c_void, block_size) };

                if bytes_readed >= 0 {
                    Ok(bytes_readed as usize)
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Unable to read data",
                    ))
                }
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Zip file is not open",
            )),
        }
    }
}

impl tokio::io::AsyncRead for Entry<'_> {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.file {
            Some(zip_file) => {
                let size = buf.remaining();
                let mut buffer: Vec<u8> = Vec::with_capacity(size);
                let bytes_read = unsafe {
                    zip_fread(
                        zip_file,
                        buffer.as_mut_ptr() as *mut c_void,
                        buffer.capacity() as u64,
                    )
                };

                if bytes_read > 0 {
                    buf.put_slice(buffer.as_slice());
                }

                Poll::Ready(Ok(()))
            }
            None => Poll::Ready(Err(tokio::io::Error::new(
                tokio::io::ErrorKind::Other,
                "Zip file is not open",
            ))),
        }
    }
}

impl Drop for Entry<'_> {
    fn drop(&mut self) {
        self.close();
    }
}

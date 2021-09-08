use std::{collections::HashMap, io, path};

use crate::{
    conf::{Cache, Conf},
    Context, GameError, GameResult,
};
use std::panic::panic_any;

#[derive(Debug, Clone)]
pub struct File {
    pub bytes: io::Cursor<Vec<u8>>,
}

impl io::Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.bytes.read(buf)
    }
}

/// A structure that contains the filesystem state and cache.
#[derive(Debug)]
pub struct Filesystem {
    root: Option<path::PathBuf>,
    files: HashMap<path::PathBuf, File>,
}

impl Filesystem {
    pub(crate) fn new(conf: &Conf) -> Filesystem {
        let mut files = HashMap::new();

        if let Cache::Tar(ref tar_file) = conf.cache {
            let mut archive = tar::Archive::new(tar_file.as_slice());

            for file in archive.entries().unwrap_or_else(|e| panic_any(e)) {
                use std::io::Read;

                let mut file = file.unwrap_or_else(|e| panic_any(e));
                let filename =
                    std::path::PathBuf::from(file.path().unwrap_or_else(|e| panic_any(e)));
                let mut buf = vec![];

                file.read_to_end(&mut buf).unwrap_or_else(|e| panic_any(e));
                if !buf.is_empty() {
                    files.insert(
                        filename,
                        File {
                            bytes: io::Cursor::new(buf),
                        },
                    );
                }
            }
        }

        let root = conf.physical_root_dir.clone();
        Filesystem { root, files }
    }

    /// Opens the given `path` and returns the resulting `File`
    /// in read-only mode.
    pub fn open<P: AsRef<path::Path>>(&mut self, path: P) -> GameResult<File> {
        let mut path = path::PathBuf::from(path.as_ref());

        // workaround for ggez-style pathes: in ggez paths starts with "/", while in the cache
        // dictionary they are presented without "/"
        if let Ok(stripped) = path.strip_prefix("/") {
            path = path::PathBuf::from(stripped);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(ref root_path) = self.root {
                if let Ok(buf) = std::fs::read(root_path.join(&path)) {
                    let bytes = io::Cursor::new(buf);
                    return Ok(File { bytes });
                }
            }
        }

        if !self.files.contains_key(&path) {
            return Err(GameError::FilesystemError(format!(
                "No such file: {:?}",
                &path
            )));
        }
        Ok(self.files[&path].clone())
    }
}

/// Opens the given path and returns the resulting `File`
/// in read-only mode.
pub fn open<P: AsRef<path::Path>>(ctx: &mut Context, path: P) -> GameResult<File> {
    ctx.filesystem.open(path)
}

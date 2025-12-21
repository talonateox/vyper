use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use spin::Mutex;

use super::types::*;

#[derive(Clone)]
enum Node {
    File(Vec<u8>),
    Directory(BTreeMap<String, Node>),
}

impl Node {
    fn as_dir(&self) -> Option<&BTreeMap<String, Node>> {
        match self {
            Node::Directory(entries) => Some(entries),
            _ => None,
        }
    }

    fn as_dir_mut(&mut self) -> Option<&mut BTreeMap<String, Node>> {
        match self {
            Node::Directory(entries) => Some(entries),
            _ => None,
        }
    }

    fn as_file(&self) -> Option<&Vec<u8>> {
        match self {
            Node::File(data) => Some(data),
            _ => None,
        }
    }

    fn as_file_mut(&mut self) -> Option<&mut Vec<u8>> {
        match self {
            Node::File(data) => Some(data),
            _ => None,
        }
    }

    fn file_type(&self) -> FileType {
        match self {
            Node::File(_) => FileType::File,
            Node::Directory(_) => FileType::Directory,
        }
    }

    fn size(&self) -> usize {
        match self {
            Node::File(data) => data.len(),
            Node::Directory(entries) => entries.len(),
        }
    }
}

pub struct TmpFs {
    root: Mutex<Node>,
}

impl TmpFs {
    pub fn new() -> Self {
        Self {
            root: Mutex::new(Node::Directory(BTreeMap::new())),
        }
    }

    fn path_parts(path: &str) -> Vec<&str> {
        path.split('/').filter(|s| !s.is_empty()).collect()
    }

    fn navigate_to_parent<'a>(
        root: &'a mut Node,
        parts: &[&'a str],
    ) -> VfsResult<(&'a mut Node, &'a str)> {
        if parts.is_empty() {
            return Err(VfsError::InvalidPath);
        }

        let (parent_parts, name) = parts.split_at(parts.len() - 1);
        let mut current = root;

        for part in parent_parts {
            current = current
                .as_dir_mut()
                .ok_or(VfsError::NotADirectory)?
                .get_mut(*part)
                .ok_or(VfsError::NotFound)?;
        }

        Ok((current, name[0]))
    }

    fn navigate<'a>(root: &'a Node, parts: &[&str]) -> VfsResult<&'a Node> {
        let mut current = root;

        for part in parts {
            current = current
                .as_dir()
                .ok_or(VfsError::NotADirectory)?
                .get(*part)
                .ok_or(VfsError::NotFound)?;
        }

        Ok(current)
    }

    fn navigate_mut<'a>(root: &'a mut Node, parts: &[&str]) -> VfsResult<&'a mut Node> {
        let mut current = root;

        for part in parts {
            current = current
                .as_dir_mut()
                .ok_or(VfsError::NotADirectory)?
                .get_mut(*part)
                .ok_or(VfsError::NotFound)?;
        }

        Ok(current)
    }
}

impl Filesystem for TmpFs {
    fn open(&self, path: &str, flags: OpenFlags) -> VfsResult<Box<dyn FileHandle>> {
        let mut root = self.root.lock();
        let parts = Self::path_parts(path);

        if parts.is_empty() {
            return Err(VfsError::IsADirectory);
        }

        let node_result = Self::navigate_mut(&mut root, &parts);

        match node_result {
            Ok(node) => {
                if node.file_type() == FileType::Directory {
                    return Err(VfsError::IsADirectory);
                }

                let data = node.as_file().unwrap().clone();
                let data = if flags.truncate { Vec::new() } else { data };
                let position = if flags.append { data.len() } else { 0 };

                Ok(Box::new(TmpFileHandle {
                    data,
                    position,
                    path: path.to_string(),
                    flags,
                    fs: self as *const TmpFs,
                }))
            }
            Err(VfsError::NotFound) if flags.create => {
                let (parent, name) = Self::navigate_to_parent(&mut root, &parts)?;
                let dir = parent.as_dir_mut().ok_or(VfsError::NotADirectory)?;
                dir.insert(name.to_string(), Node::File(Vec::new()));

                Ok(Box::new(TmpFileHandle {
                    data: Vec::new(),
                    position: 0,
                    path: path.to_string(),
                    flags,
                    fs: self as *const TmpFs,
                }))
            }
            Err(e) => Err(e),
        }
    }

    fn mkdir(&self, path: &str) -> VfsResult<()> {
        let mut root = self.root.lock();
        let parts = Self::path_parts(path);

        if parts.is_empty() {
            return Err(VfsError::AlreadyExists);
        }

        let (parent, name) = Self::navigate_to_parent(&mut root, &parts)?;
        let dir = parent.as_dir_mut().ok_or(VfsError::NotADirectory)?;

        if dir.contains_key(name) {
            return Err(VfsError::AlreadyExists);
        }

        dir.insert(name.to_string(), Node::Directory(BTreeMap::new()));
        Ok(())
    }

    fn remove(&self, path: &str) -> VfsResult<()> {
        let mut root = self.root.lock();
        let parts = Self::path_parts(path);

        if parts.is_empty() {
            return Err(VfsError::PermissionDenied);
        }

        let (parent, name) = Self::navigate_to_parent(&mut root, &parts)?;
        let dir = parent.as_dir_mut().ok_or(VfsError::NotADirectory)?;

        match dir.get(name) {
            Some(Node::File(_)) => {
                dir.remove(name);
                Ok(())
            }
            Some(Node::Directory(_)) => Err(VfsError::IsADirectory),
            None => Err(VfsError::NotFound),
        }
    }

    fn rmdir(&self, path: &str) -> VfsResult<()> {
        let mut root = self.root.lock();
        let parts = Self::path_parts(path);

        if parts.is_empty() {
            return Err(VfsError::PermissionDenied);
        }

        let (parent, name) = Self::navigate_to_parent(&mut root, &parts)?;
        let dir = parent.as_dir_mut().ok_or(VfsError::NotADirectory)?;

        match dir.get(name) {
            Some(Node::Directory(entries)) if entries.is_empty() => {
                dir.remove(name);
                Ok(())
            }
            Some(Node::Directory(_)) => Err(VfsError::NotEmpty),
            Some(Node::File(_)) => Err(VfsError::NotADirectory),
            None => Err(VfsError::NotFound),
        }
    }

    fn readdir(&self, path: &str) -> VfsResult<Vec<DirEntry>> {
        let root = self.root.lock();
        let parts = Self::path_parts(path);

        let node = if parts.is_empty() {
            &*root
        } else {
            Self::navigate(&root, &parts)?
        };

        let dir = node.as_dir().ok_or(VfsError::NotADirectory)?;

        Ok(dir
            .iter()
            .map(|(name, node)| DirEntry {
                name: name.clone(),
                file_type: node.file_type(),
            })
            .collect())
    }

    fn metadata(&self, path: &str) -> VfsResult<Metadata> {
        let root = self.root.lock();
        let parts = Self::path_parts(path);

        let node = if parts.is_empty() {
            &*root
        } else {
            Self::navigate(&root, &parts)?
        };

        Ok(Metadata {
            file_type: node.file_type(),
            size: node.size(),
        })
    }
}

struct TmpFileHandle {
    data: Vec<u8>,
    position: usize,
    path: String,
    flags: OpenFlags,
    fs: *const TmpFs,
}

unsafe impl Send for TmpFileHandle {}
unsafe impl Sync for TmpFileHandle {}

impl FileHandle for TmpFileHandle {
    fn read(&mut self, buf: &mut [u8]) -> VfsResult<usize> {
        if !self.flags.read {
            return Err(VfsError::PermissionDenied);
        }

        let available = self.data.len().saturating_sub(self.position);
        let to_read = buf.len().min(available);

        buf[..to_read].copy_from_slice(&self.data[self.position..self.position + to_read]);
        self.position += to_read;

        Ok(to_read)
    }

    fn write(&mut self, buf: &[u8]) -> VfsResult<usize> {
        if !self.flags.write {
            return Err(VfsError::PermissionDenied);
        }

        if self.flags.append {
            self.position = self.data.len();
        }

        let end_position = self.position + buf.len();

        if end_position > self.data.len() {
            self.data.resize(end_position, 0);
        }

        self.data[self.position..end_position].copy_from_slice(buf);
        self.position = end_position;

        unsafe {
            let fs = &*self.fs;
            let mut root = fs.root.lock();
            let parts = TmpFs::path_parts(&self.path);
            if let Ok(node) = TmpFs::navigate_mut(&mut root, &parts) {
                if let Some(file_data) = node.as_file_mut() {
                    *file_data = self.data.clone();
                }
            }
        }

        Ok(buf.len())
    }

    fn seek(&mut self, pos: SeekFrom) -> VfsResult<usize> {
        let new_pos = match pos {
            SeekFrom::Start(n) => n as isize,
            SeekFrom::Current(n) => self.position as isize + n,
            SeekFrom::End(n) => self.data.len() as isize + n,
        };

        if new_pos < 0 {
            return Err(VfsError::InvalidPath);
        }

        self.position = new_pos as usize;
        Ok(self.position)
    }

    fn metadata(&self) -> VfsResult<Metadata> {
        Ok(Metadata {
            file_type: FileType::File,
            size: self.data.len(),
        })
    }
}

//! lwext4_core 兼容层
//!
//! 提供与 lwext4_rust 兼容的接口，使得可以无缝替换

use alloc::{string::String, vec::Vec};
use axerrno::LinuxError;
use core::time::Duration;

use super::{ArceOsHal, Ext4CoreDisk};

// ===== 类型重导出 =====

/// 文件系统配置（兼容 lwext4_rust::FsConfig）
pub use lwext4_core::FsConfig;

/// Inode 类型（兼容 lwext4_rust::InodeType）
pub use lwext4_core::InodeType;

/// 文件属性（兼容 lwext4_rust::FileAttr）
pub use lwext4_core::FileAttr;

/// SystemHal trait（兼容 lwext4_rust::SystemHal）
pub use lwext4_core::SystemHal;

/// 根目录 inode 编号
pub const EXT4_ROOT_INO: u32 = 2;

// ===== 错误类型兼容层 =====

/// Ext4 错误（兼容 lwext4_rust::Ext4Error）
#[derive(Debug, Clone)]
pub struct Ext4Error {
    pub code: i32,
    pub message: Option<&'static str>,
}

impl Ext4Error {
    pub fn new(code: i32, message: Option<&'static str>) -> Self {
        Self { code, message }
    }

    /// 从 lwext4_core::Error 转换
    fn from_core_error(err: lwext4_core::Error) -> Self {
        use lwext4_core::ErrorKind;

        let code = match err.kind() {
            ErrorKind::NotFound => LinuxError::ENOENT as i32,
            ErrorKind::PermissionDenied => LinuxError::EACCES as i32,
            ErrorKind::AlreadyExists => LinuxError::EEXIST as i32,
            ErrorKind::InvalidInput => LinuxError::EINVAL as i32,
            ErrorKind::Io => LinuxError::EIO as i32,
            ErrorKind::NoSpace => LinuxError::ENOSPC as i32,
            ErrorKind::NotEmpty => LinuxError::ENOTEMPTY as i32,
            ErrorKind::Unsupported => LinuxError::EOPNOTSUPP as i32,
            ErrorKind::Corrupted => LinuxError::EUCLEAN as i32,
            ErrorKind::Busy => LinuxError::EBUSY as i32,
            ErrorKind::InvalidState => LinuxError::EBADFD as i32,
            _ => LinuxError::EIO as i32,
        };

        Self {
            code,
            message: Some(err.message()),
        }
    }
}

/// Ext4 结果类型（兼容 lwext4_rust::Ext4Result）
pub type Ext4Result<T> = Result<T, Ext4Error>;

// ===== 目录条目兼容层 =====

/// 目录条目（兼容 lwext4_rust::DirEntry）
pub struct DirEntry {
    inner: lwext4_core::DirEntry,
}

impl DirEntry {
    pub fn ino(&self) -> u32 {
        self.inner.inode
    }

    pub fn name(&self) -> &[u8] {
        self.inner.name.as_bytes()
    }

    pub fn inode_type(&self) -> InodeType {
        // 将 u8 文件类型转换为 InodeType
        use lwext4_core::dir::write::{EXT4_DE_REG_FILE, EXT4_DE_DIR, EXT4_DE_SYMLINK};

        match self.inner.file_type {
            EXT4_DE_DIR => InodeType::Directory,
            EXT4_DE_REG_FILE => InodeType::RegularFile,
            EXT4_DE_SYMLINK => InodeType::Symlink,
            _ => InodeType::Unknown,
        }
    }
}

// ===== 查找结果 =====

/// 查找结果（兼容 lwext4_rust 的 lookup 返回值）
pub struct LookupResult {
    ino: u32,
    name: Vec<u8>,
    inode_type: InodeType,
}

impl LookupResult {
    pub fn entry(&self) -> DirEntry {
        use lwext4_core::dir::write::{EXT4_DE_REG_FILE, EXT4_DE_DIR, EXT4_DE_SYMLINK};

        let file_type = match self.inode_type {
            InodeType::Directory => EXT4_DE_DIR,
            InodeType::RegularFile => EXT4_DE_REG_FILE,
            InodeType::Symlink => EXT4_DE_SYMLINK,
            _ => 0,
        };

        DirEntry {
            inner: lwext4_core::DirEntry {
                inode: self.ino,
                name: String::from_utf8_lossy(&self.name).into_owned(),
                file_type,
            },
        }
    }
}

// ===== 目录读取器兼容层 =====

/// 目录读取结果（兼容 lwext4_rust 的 read_dir 返回值）
pub struct DirReaderResult<'a, D: lwext4_core::BlockDevice> {
    reader: lwext4_core::DirReader<'a, 'a, D>,
}

impl<'a, D: lwext4_core::BlockDevice> DirReaderResult<'a, D> {
    pub fn entry(&self) -> DirEntry {
        DirEntry {
            inner: self.reader.current().unwrap().clone(),
        }
    }

    pub fn current(&self) -> Option<DirEntry> {
        self.reader.current().map(|e| DirEntry { inner: e.clone() })
    }

    pub fn step(&mut self) -> Ext4Result<()> {
        self.reader.step().map_err(Ext4Error::from_core_error)
    }

    pub fn offset(&self) -> u64 {
        self.reader.offset()
    }
}

// ===== Inode 引用兼容层 =====

/// Inode 引用包装器（兼容 lwext4_rust 的 with_inode_ref）
pub struct InodeRefWrapper<'a, D: lwext4_core::BlockDevice> {
    inner: lwext4_core::InodeRef<'a, D>,
}

impl<'a, D: lwext4_core::BlockDevice> InodeRefWrapper<'a, D> {
    pub fn size(&self) -> u64 {
        self.inner.size().unwrap_or(0)
    }

    pub fn mode(&self) -> u32 {
        self.inner.mode().unwrap_or(0)
    }

    pub fn set_mode(&mut self, mode: u32) {
        let _ = self.inner.set_mode(mode as u16);
    }

    pub fn set_owner(&mut self, uid: u32, gid: u32) {
        let _ = self.inner.set_uid(uid);
        let _ = self.inner.set_gid(gid);
    }

    pub fn set_atime(&mut self, time: &Duration) {
        let _ = self.inner.set_atime(time.as_secs() as u32);
    }

    pub fn set_mtime(&mut self, time: &Duration) {
        let _ = self.inner.set_mtime(time.as_secs() as u32);
    }

    pub fn update_ctime(&mut self) {
        if let Some(now) = ArceOsHal::now() {
            let _ = self.inner.set_ctime(now.as_secs() as u32);
        }
    }
}

// ===== 文件系统统计信息 =====

/// 文件系统统计信息（兼容 lwext4_rust）
#[derive(Debug, Clone, Default)]
pub struct FsStat {
    pub block_size: u32,
    pub blocks_count: u64,
    pub free_blocks_count: u64,
    pub inodes_count: u32,
    pub free_inodes_count: u32,
}

// ===== 主文件系统封装 =====

/// Ext4 文件系统（兼容 lwext4_rust::Ext4Filesystem）
pub struct Ext4Filesystem<H: SystemHal, D: lwext4_core::BlockDevice> {
    inner: lwext4_core::Ext4FileSystem<D>,
    _phantom: core::marker::PhantomData<H>,
}

impl<H: SystemHal, D: lwext4_core::BlockDevice> Ext4Filesystem<H, D> {
    /// 创建新的文件系统实例（兼容 lwext4_rust）
    pub fn new(device: D, _config: FsConfig) -> Ext4Result<Self> {
        // 将设备包装为 BlockDev
        let bdev = lwext4_core::BlockDev::new(device);

        let inner = lwext4_core::Ext4FileSystem::mount(bdev)
            .map_err(Ext4Error::from_core_error)?;

        Ok(Self {
            inner,
            _phantom: core::marker::PhantomData,
        })
    }

    /// 获取文件系统统计信息
    pub fn stat(&mut self) -> Ext4Result<FsStat> {
        let stats = self.inner.stats().map_err(Ext4Error::from_core_error)?;

        Ok(FsStat {
            block_size: stats.block_size,
            blocks_count: stats.blocks_total,
            free_blocks_count: stats.blocks_free,
            inodes_count: stats.inodes_total,
            free_inodes_count: stats.free_inodes_count,
        })
    }

    /// 刷新文件系统
    pub fn flush(&mut self) -> Ext4Result<()> {
        // lwext4_core 的写入是同步的，不需要显式 flush
        Ok(())
    }

    /// 查找目录项
    ///
    /// 注意：这个方法返回 DirReaderResult 而不是单个 DirEntry
    /// 这是为了兼容 lwext4_rust 的 API
    pub fn lookup(&mut self, dir_ino: u32, name: &str) -> Ext4Result<LookupResult> {
        let child_ino = self
            .inner
            .lookup_in_dir(dir_ino, name)
            .map_err(Ext4Error::from_core_error)?;

        // 获取文件属性来构造完整的 DirEntry
        let attr = self
            .inner
            .get_inode_attr(child_ino)
            .map_err(Ext4Error::from_core_error)?;

        Ok(LookupResult {
            ino: child_ino,
            name: name.as_bytes().to_vec(),
            inode_type: attr.node_type,
        })
    }

    /// 获取文件属性
    pub fn get_attr(&mut self, ino: u32, attr: &mut FileAttr) -> Ext4Result<()> {
        let file_attr = self
            .inner
            .get_inode_attr(ino)
            .map_err(Ext4Error::from_core_error)?;

        *attr = file_attr;
        Ok(())
    }

    /// 读取文件数据
    pub fn read_at(&mut self, ino: u32, buf: &mut [u8], offset: u64) -> Ext4Result<usize> {
        self.inner
            .read_at_inode(ino, offset, buf)
            .map_err(Ext4Error::from_core_error)
    }

    /// 写入文件数据
    pub fn write_at(&mut self, ino: u32, buf: &[u8], offset: u64) -> Ext4Result<usize> {
        self.inner
            .write_at_inode(ino, offset, buf)
            .map_err(Ext4Error::from_core_error)
    }

    /// 设置文件大小
    pub fn set_len(&mut self, ino: u32, len: u64) -> Ext4Result<()> {
        self.inner
            .truncate_inode(ino, len)
            .map_err(Ext4Error::from_core_error)
    }

    /// 设置符号链接
    pub fn set_symlink(&mut self, _ino: u32, _target: &[u8]) -> Ext4Result<()> {
        // TODO: 实现符号链接
        Err(Ext4Error::new(
            LinuxError::EOPNOTSUPP as i32,
            Some("Symlink not yet implemented"),
        ))
    }

    /// 读取目录
    pub fn read_dir(&mut self, dir_ino: u32, offset: u64) -> Ext4Result<DirReaderResult<'_, D>> {
        // 使用 read_dir_from_inode API
        let reader = self
            .inner
            .read_dir_from_inode(dir_ino, offset)
            .map_err(Ext4Error::from_core_error)?;

        Ok(DirReaderResult { reader })
    }

    /// 创建文件或目录
    pub fn create(
        &mut self,
        parent_ino: u32,
        name: &str,
        inode_type: InodeType,
        mode: u32,
    ) -> Ext4Result<u32> {
        use lwext4_core::dir::write::{EXT4_DE_REG_FILE, EXT4_DE_DIR, EXT4_DE_SYMLINK};

        let file_type = match inode_type {
            InodeType::RegularFile => EXT4_DE_REG_FILE,
            InodeType::Directory => EXT4_DE_DIR,
            InodeType::Symlink => EXT4_DE_SYMLINK,
            _ => {
                return Err(Ext4Error::new(
                    LinuxError::EOPNOTSUPP as i32,
                    Some("Unsupported inode type"),
                ))
            }
        };

        self.inner
            .create_in_dir(parent_ino, name, file_type, mode as u16)
            .map_err(Ext4Error::from_core_error)
    }

    /// 删除文件
    pub fn unlink(&mut self, parent_ino: u32, name: &str) -> Ext4Result<()> {
        self.inner
            .unlink_from_dir(parent_ino, name)
            .map_err(Ext4Error::from_core_error)
    }

    /// 重命名
    pub fn rename(
        &mut self,
        src_parent: u32,
        src_name: &str,
        dst_parent: u32,
        dst_name: &str,
    ) -> Ext4Result<()> {
        self.inner
            .rename_inode(src_parent, src_name, dst_parent, dst_name)
            .map_err(Ext4Error::from_core_error)
    }

    /// 创建硬链接
    pub fn link(&mut self, parent_ino: u32, name: &str, target_ino: u32) -> Ext4Result<()> {
        self.inner
            .link_inode(parent_ino, name, target_ino)
            .map_err(Ext4Error::from_core_error)
    }

    /// 操作 inode 引用
    pub fn with_inode_ref<F, R>(&mut self, ino: u32, f: F) -> Ext4Result<R>
    where
        F: FnOnce(&mut InodeRefWrapper<'_, D>) -> Ext4Result<R>,
    {
        self.inner
            .with_inode_ref(ino, |inode_ref| {
                let mut wrapper = InodeRefWrapper { inner: inode_ref };
                f(&mut wrapper)
            })
            .map_err(Ext4Error::from_core_error)
    }
}

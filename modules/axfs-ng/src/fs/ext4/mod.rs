mod fs;
mod inode;
mod util;

// lwext4_core 支持（新增）
mod hal;
mod adapter;
pub mod wrapper;

#[allow(unused_imports)]
use axdriver::{AxBlockDevice, prelude::BlockDriverOps};
pub use fs::*;
pub use inode::*;
use lwext4_rust::{BlockDevice, Ext4Error, Ext4Result, ffi::EIO};

// 导出 lwext4_core 相关类型
pub use hal::ArceOsHal;
pub use adapter::Ext4CoreDisk;

// 导出 wrapper（可选，用于测试）
#[allow(unused)]
pub use wrapper as lwext4_core_compat;

pub(crate) struct Ext4Disk(AxBlockDevice);

impl BlockDevice for Ext4Disk {
    fn read_blocks(&mut self, block_id: u64, buf: &mut [u8]) -> Ext4Result<usize> {
        self.0
            .read_block(block_id, buf)
            .map_err(|_| Ext4Error::new(EIO as _, None))?;
        Ok(buf.len())
    }

    fn write_blocks(&mut self, block_id: u64, buf: &[u8]) -> Ext4Result<usize> {
        self.0
            .write_block(block_id, buf)
            .map_err(|_| Ext4Error::new(EIO as _, None))?;
        Ok(buf.len())
    }

    fn num_blocks(&self) -> Ext4Result<u64> {
        Ok(self.0.num_blocks())
    }
}
